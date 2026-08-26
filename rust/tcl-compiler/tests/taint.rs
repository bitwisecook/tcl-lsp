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

//! Taint-analysis tests.
//!
//! These drive the taint pass and assert over the returned `TaintWarning` list
//! (`.code` / `.variable` / `.sink_command` / `.message`), plus a block of
//! pure-lattice tests over `TaintLattice` / `TaintColour` / `taint_join`. The
//! driver is [`tcl_compiler::taint::find_taint_warnings_for_cu`] (the
//! whole-unit taint pass that `run_all_checks` also drives), over the `pub`
//! [`tcl_compiler::taint::TaintLattice`] / [`TaintColour`] types.
//!
//! ## Driving a snippet
//!
//! [`warns`] builds a [`CompilationUnit`] for the snippet and runs
//! `find_taint_warnings_for_cu`, the `TaintWarning`-producing surface (sink
//! detection T100–T106, setter constraints IRULE3101, iRules URI-split
//! IRULE3103, destructive-file W313). [`codes`] returns the sorted diagnostic
//! codes; [`of_code`] filters warnings by diagnostic code. The lattice join is
//! [`TaintLattice::join`].
//!
//! ## Dialect handling
//!
//! The dialect is threaded explicitly into `find_taint_warnings_for_cu`, so
//! each test passes the dialect it needs:
//!   * Dialect-agnostic tests run under [`D`] (`tcl8.6`).
//!     iRules *sources* (`HTTP::uri`, `IP::client_addr`, …) still taint there —
//!     they are registered in every dialect (registry test
//!     `http_uri_is_a_source_in_every_dialect`) — so the T100/T101/T102/T103
//!     source+sink tests work under plain Tcl.
//!   * iRules *sinks* (IRULE3001–3004, `log` → IRULE3003) only fire under
//!     [`IR`] (`f5-irules`); those tests pass `IR`.
//!   * The `…_not_in_tcl86` / `…_tcl_dialect_clean` cases pass `D`
//!     explicitly to prove the iRules sink does NOT fire under plain Tcl.
//!
//! ## Policy-vs-semantic proof split (C-Tcl)
//!
//! Taint is a *security/policy* analysis, not a runtime value. Most assertions
//! are policy-level — "this source is tainted", "this sink is flagged", "this
//! colour suppresses that code" — and have no tclsh ground truth to pin: tclsh
//! does not implement taint, so there is nothing to compare against. Those are
//! pure policy checks.
//!
//! Where a test's *premise* is a Tcl-semantic fact — a command's actual
//! behaviour, a string transform, whether the first list element becomes the
//! command word — it is verified against `scripts/dev/tclsh_check.sh` (tclsh8.6
//! + tclsh9.0) and cited in a `// tclsh:` comment.
//!
//! The headline facts proven while authoring this file:
//!
//!   * `eval [list $raw]`        → tclsh runs `$raw` as the command word
//!     (`proc marker args {puts EXECUTED}; eval [list marker]` prints
//!     `EXECUTED`) — so `LIST_CANONICAL` does NOT suppress T100 (D5-T100).
//!   * `eval [list puts $raw]`   → tclsh prints `marker` (`puts` is the command
//!     word, `$raw` is its argument) — so a literal `[list <known-cmd> …]` head
//!     DOES suppress T100.
//!   * `string length $x`        → tclsh returns an int (`11` for an 11-char
//!     value) — confirming it is a taint sanitiser (fixed numeric return).
//!   * `regexp -- …`             → tclsh treats `--` as the option terminator
//!     (`regexp -- -foo bar` → `0`, not a `bad switch` error).
//!
//! F5/iRules-specific taint sources and sinks (`HTTP::*`, `IP::*`, `SSL::*`,
//! `HTTP::respond`, …) are NOT core-tclsh commands, so snippets that mention
//! them carry a `// f5-dialect` note — there is no tclsh proof to cite for the
//! command itself, only the policy verdict.

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::taint::{TaintColour, TaintLattice, TaintWarning, find_taint_warnings_for_cu};
use tcl_registry::model::ingress::static_context_for;

/// Default dialect for snippets that are not dialect-sensitive. iRules sources
/// still taint under this dialect (registered globally), so the generic
/// T100–T106 tests use it.
const D: &str = "tcl8.6";
/// iRules dialect — gates the IRULE3001–3004 / IRULE3101 / IRULE3103 sinks and
/// `log` → IRULE3003.
const IR: &str = "f5-irules";

/// Every `TaintWarning` the whole-unit taint pass surfaces for `src` under
/// `dialect`.
fn warns(src: &str, dialect: &str) -> Vec<TaintWarning> {
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    find_taint_warnings_for_cu(&cu, registry, dialect_opt)
}

/// Sorted list of diagnostic codes from the taint pass.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = warns(src, dialect)
        .iter()
        .map(|w| w.code.to_string())
        .collect();
    out.sort();
    out
}

/// Taint warnings of one diagnostic `code`.
fn of_code(src: &str, dialect: &str, code: &str) -> Vec<TaintWarning> {
    warns(src, dialect)
        .into_iter()
        .filter(|w| w.code.to_string() == code)
        .collect()
}

// ===========================================================================
// Lattice join semantics (`taint_join` → `TaintLattice::join`).
//
// Pure lattice algebra: taint is a may-have (union), mitigating colours are a
// must-have (intersection). No tclsh analogue — this is the analyser's internal
// data structure.
// ===========================================================================
mod taint_join {
    use super::*;

    fn lat(colour: TaintColour) -> TaintLattice {
        TaintLattice { colours: colour }
    }
    const UNTAINTED: TaintColour = TaintColour::empty();
    const TAINTED: TaintColour = TaintColour::TAINTED;

    fn join(a: TaintColour, b: TaintColour) -> TaintLattice {
        lat(a).join(lat(b))
    }

    #[test]
    fn untainted_untainted() {
        assert!(!join(UNTAINTED, UNTAINTED).is_tainted());
    }

    #[test]
    fn tainted_untainted() {
        assert!(join(TAINTED, UNTAINTED).is_tainted());
    }

    #[test]
    fn untainted_tainted() {
        assert!(join(UNTAINTED, TAINTED).is_tainted());
    }

    #[test]
    fn tainted_tainted() {
        assert!(join(TAINTED, TAINTED).is_tainted());
    }

    #[test]
    fn path_prefixed_join_preserves_colour() {
        let p = TAINTED | TaintColour::PATH_PREFIXED;
        let r = join(p, p);
        assert!(r.is_tainted());
        assert!(r.colours.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn path_prefixed_join_with_generic_loses_colour() {
        let r = join(TAINTED | TaintColour::PATH_PREFIXED, TAINTED);
        assert!(r.is_tainted());
        assert!(!r.colours.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn path_prefixed_join_with_untainted() {
        // Joining PATH with untainted returns PATH unchanged (untainted is the
        // join *identity*), so PATH_PREFIXED survives.
        let r = join(TAINTED | TaintColour::PATH_PREFIXED, UNTAINTED);
        assert!(r.is_tainted());
        // FIXED: clean is now the join identity (not a mitigation annihilator),
        // so PATH_PREFIXED survives.
        assert!(r.colours.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn ip_join_preserves_colour() {
        let ip = TAINTED | TaintColour::IP_ADDRESS;
        let r = join(ip, ip);
        assert!(r.is_tainted());
        assert!(r.colours.contains(TaintColour::IP_ADDRESS));
    }

    #[test]
    fn ip_join_with_generic_loses_colour() {
        let r = join(TAINTED | TaintColour::IP_ADDRESS, TAINTED);
        assert!(r.is_tainted());
        assert!(!r.colours.contains(TaintColour::IP_ADDRESS));
    }

    #[test]
    fn port_join_preserves_colour() {
        let port = TAINTED | TaintColour::PORT;
        let r = join(port, port);
        assert!(r.is_tainted());
        assert!(r.colours.contains(TaintColour::PORT));
    }

    #[test]
    fn fqdn_join_with_untainted() {
        let r = join(TAINTED | TaintColour::FQDN, UNTAINTED);
        assert!(r.is_tainted());
        // FIXED: clean is the join identity, so FQDN survives.
        assert!(r.colours.contains(TaintColour::FQDN));
    }

    #[test]
    fn different_colours_lose_both() {
        // IP_ADDRESS & PORT = 0 (different flags) → join keeps neither.
        let r = join(
            TAINTED | TaintColour::IP_ADDRESS,
            TAINTED | TaintColour::PORT,
        );
        assert!(r.is_tainted());
        assert!(!r.colours.contains(TaintColour::IP_ADDRESS));
        assert!(!r.colours.contains(TaintColour::PORT));
    }
}

// ===========================================================================
// Join semantics for the extended colour set.
// ===========================================================================
mod lattice_join_new_colours {
    use super::*;

    fn lat(colour: TaintColour) -> TaintLattice {
        TaintLattice { colours: colour }
    }
    const TAINTED: TaintColour = TaintColour::TAINTED;
    fn join(a: TaintColour, b: TaintColour) -> TaintLattice {
        lat(a).join(lat(b))
    }
    fn t(c: TaintColour) -> TaintColour {
        TAINTED | c
    }

    #[test]
    fn crlf_free_self() {
        let r = join(t(TaintColour::CRLF_FREE), t(TaintColour::CRLF_FREE));
        assert!(r.is_tainted() && r.colours.contains(TaintColour::CRLF_FREE));
    }

    #[test]
    fn crlf_free_with_generic_loses() {
        let r = join(t(TaintColour::CRLF_FREE), TAINTED);
        assert!(r.is_tainted() && !r.colours.contains(TaintColour::CRLF_FREE));
    }

    #[test]
    fn crlf_free_with_untainted() {
        let r = join(t(TaintColour::CRLF_FREE), TaintColour::empty());
        assert!(r.is_tainted());
        // FIXED: clean is the join identity, so CRLF_FREE survives.
        assert!(r.colours.contains(TaintColour::CRLF_FREE));
    }

    #[test]
    fn shell_atom_self() {
        let r = join(t(TaintColour::SHELL_ATOM), t(TaintColour::SHELL_ATOM));
        assert!(r.is_tainted() && r.colours.contains(TaintColour::SHELL_ATOM));
    }

    #[test]
    fn shell_atom_with_generic_loses() {
        let r = join(t(TaintColour::SHELL_ATOM), TAINTED);
        assert!(!r.colours.contains(TaintColour::SHELL_ATOM));
    }

    #[test]
    fn list_canonical_self() {
        let r = join(
            t(TaintColour::LIST_CANONICAL),
            t(TaintColour::LIST_CANONICAL),
        );
        assert!(r.colours.contains(TaintColour::LIST_CANONICAL));
    }

    #[test]
    fn regex_literal_with_html_escaped_loses_both() {
        let r = join(t(TaintColour::REGEX_LITERAL), t(TaintColour::HTML_ESCAPED));
        assert!(!r.colours.contains(TaintColour::REGEX_LITERAL));
        assert!(!r.colours.contains(TaintColour::HTML_ESCAPED));
    }

    #[test]
    fn path_normalised_self() {
        let r = join(
            t(TaintColour::PATH_NORMALISED),
            t(TaintColour::PATH_NORMALISED),
        );
        assert!(r.colours.contains(TaintColour::PATH_NORMALISED));
    }

    #[test]
    fn header_token_safe_self() {
        let r = join(
            t(TaintColour::HEADER_TOKEN_SAFE),
            t(TaintColour::HEADER_TOKEN_SAFE),
        );
        assert!(r.colours.contains(TaintColour::HEADER_TOKEN_SAFE));
    }

    #[test]
    fn html_escaped_self() {
        let r = join(t(TaintColour::HTML_ESCAPED), t(TaintColour::HTML_ESCAPED));
        assert!(r.colours.contains(TaintColour::HTML_ESCAPED));
    }

    #[test]
    fn url_encoded_self() {
        let r = join(t(TaintColour::URL_ENCODED), t(TaintColour::URL_ENCODED));
        assert!(r.colours.contains(TaintColour::URL_ENCODED));
    }

    #[test]
    fn url_encoded_with_generic_loses() {
        let r = join(t(TaintColour::URL_ENCODED), TAINTED);
        assert!(!r.colours.contains(TaintColour::URL_ENCODED));
    }

    #[test]
    fn multi_colour_join_preserves_shared() {
        let both = t(TaintColour::CRLF_FREE | TaintColour::URL_ENCODED);
        let r = join(both, both);
        assert!(r.colours.contains(TaintColour::CRLF_FREE));
        assert!(r.colours.contains(TaintColour::URL_ENCODED));
    }

    #[test]
    fn multi_colour_partial_overlap() {
        let a = t(TaintColour::CRLF_FREE | TaintColour::URL_ENCODED);
        let r = join(a, t(TaintColour::CRLF_FREE));
        assert!(r.colours.contains(TaintColour::CRLF_FREE));
        assert!(!r.colours.contains(TaintColour::URL_ENCODED));
    }
}

// ===========================================================================
// Clean code produces no taint warnings.
//
// tclsh: each snippet is ordinary, side-effect-free Tcl with no attacker
// source, so flagging it would be a false positive.
// ===========================================================================
mod no_false_positives {
    use super::*;

    #[test]
    fn literal_set() {
        assert_eq!(codes("set x 42", D), Vec::<String>::new());
    }

    #[test]
    fn incr() {
        assert_eq!(codes("set x 0\nincr x", D), Vec::<String>::new());
    }

    #[test]
    fn safe_eval() {
        // `eval $x` where x is a literal string — no taint reaches the sink.
        assert_eq!(
            codes("set x \"puts hello\"\neval $x", D),
            Vec::<String>::new()
        );
    }

    #[test]
    fn safe_expr() {
        assert_eq!(codes("set x 42\nexpr {$x + 1}", D), Vec::<String>::new());
    }

    #[test]
    fn list_operations() {
        assert_eq!(
            codes("set x [list a b c]\nset n [llength $x]", D),
            Vec::<String>::new()
        );
    }
}

// ===========================================================================
// Tcl-core I/O commands taint their results (T100 at the
// eval sink). `read`/`gets`/`exec`/`socket`/`chan` are real tclsh commands.
// ===========================================================================
mod tcl_taint_sources {
    use super::*;

    #[test]
    fn read_taints() {
        // tclsh: `read $fd` returns channel contents — attacker-influenced I/O.
        let ws = of_code("set data [read $fd]\neval $data", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "data");
    }

    #[test]
    fn gets_taints() {
        // tclsh: `gets $fd` (no varName) returns the line read from the channel.
        let ws = of_code("set line [gets $fd]\neval $line", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "line");
    }

    #[test]
    fn constant_variable_command_head_reaches_sink_at_the_use_site() {
        let ws = of_code(
            "set command eval\nset line [gets stdin]\n$command $line",
            D,
            "T100",
        );
        assert!(
            ws.iter().any(|warning| warning.variable == "line"),
            "a phase-correct constant command head must retain eval's sink semantics: {ws:?}"
        );
    }

    #[test]
    fn eval_braced_nested_command_substitution_reaches_inner_sink() {
        for source in [
            "set x [gets stdin]\neval {[eval $x]}",
            r"set x [gets stdin]
[e\166al $x]",
            "set x [gets stdin]\n[{eval} $x]",
            "set x [gets stdin]\n[\"eval\" $x]",
        ] {
            let ws = of_code(source, D, "T100");
            assert!(
                ws.iter().any(|warning| warning.variable == "x"),
                "a variable in a nested statically-headed eval reaches its sink: {source:?}: {ws:?}"
            );
        }
    }

    #[test]
    fn braced_literal_command_name_does_not_execute_its_bracket_text() {
        assert!(
            of_code("set x [gets stdin]\n{[eval $x]}", D, "T100").is_empty(),
            "a braced command name is literal; its bracket text is never evaluated"
        );
    }

    #[test]
    fn quoted_whole_command_substitution_reaches_inner_sink() {
        let ws = of_code("set x [gets stdin]\n\"[eval $x]\"", D, "T100");
        assert!(
            ws.iter().any(|warning| warning.variable == "x"),
            "quotes do not suppress a whole command substitution in command position: {ws:?}"
        );
    }

    #[test]
    fn escaped_static_command_head_obeys_release_grammar() {
        let source = r"set x [gets stdin]
[e\x76al $x]";
        assert!(
            of_code(source, "tcl8.4", "T100").is_empty(),
            "Tcl 8.4 consumes more than two hex digits, so this command is not eval"
        );
        assert!(
            of_code(source, "tcl8.6", "T100")
                .iter()
                .any(|warning| warning.variable == "x"),
            "Tcl 8.6 consumes two hex digits, so this command is eval"
        );
    }

    #[test]
    fn static_expanded_command_head_preserves_argv_semantics() {
        let positive = of_code("set x [gets stdin]\n{*}{{eval} safe} $x", "tcl9.0", "T100");
        assert!(
            positive.iter().any(|warning| warning.variable == "x"),
            "the first expanded list element is the command and the rest prefix its argv: {positive:?}"
        );
        assert!(
            of_code("set x [gets stdin]\n{*}{{puts} safe} $x", "tcl9.0", "T100",).is_empty(),
            "an expanded non-sink command head must stay clean"
        );
        assert!(
            !of_code("set x [gets stdin]\n{*}{} eval $x", "tcl9.0", "T100",).is_empty(),
            "an empty leading expansion removes itself and exposes the next command word"
        );
    }

    #[test]
    fn reaching_list_value_expands_into_command_and_argv() {
        for source in [
            "set prefix [list eval safe]\nset x [gets stdin]\n{*}$prefix $x",
            "set prefix {}\nset x [gets stdin]\n{*}$prefix eval $x",
            "if {$flag} {set prefix [list eval safe]} else {set prefix [list puts safe]}\nset x [gets stdin]\n{*}$prefix $x",
        ] {
            assert!(
                !of_code(source, "tcl9.0", "T100").is_empty(),
                "a reaching list constant must retain expanded dispatch semantics: {source}"
            );
        }
        assert!(
            of_code(
                "set prefix [list puts safe]\nset x [gets stdin]\n{*}$prefix $x",
                "tcl9.0",
                "T100",
            )
            .is_empty(),
            "a reaching expanded non-sink must stay clean"
        );
    }

    #[test]
    fn unknown_expansion_arms_do_not_erase_a_reaching_sink() {
        for source in [
            r"if {$flag} {set command eval} else {set command \x7b}
set x [gets stdin]
{*}$command $x",
            "if {$flag} {set command eval} else {set command {}}\nset next puts\nset x [gets stdin]\n{*}$command $next $x",
        ] {
            assert!(
                !of_code(source, "tcl9.0", "T100").is_empty(),
                "a malformed or unresolved expansion arm must not erase another arm's eval sink: {source}"
            );
        }
    }

    #[test]
    fn nested_command_head_sink_does_not_consume_outer_arguments() {
        for (source, code) in [
            ("set x [gets stdin]\n[eval {string cat puts}] $x", "T100"),
            ("set x [gets stdin]\n[puts {$x}] $x", "T101"),
        ] {
            assert!(
                of_code(source, D, code).is_empty(),
                "a nested sink must not consume a braced literal or a later outer argument: {source:?}"
            );
        }
    }

    #[test]
    fn whole_command_substitution_classifies_each_executed_inner_sink() {
        for (source, code) in [
            ("set x [gets stdin]\n[puts $x]", "T101"),
            ("set x [gets stdin]\n[puts safe; eval $x]", "T100"),
            ("set x [gets stdin]\nprefix[eval $x]", "T100"),
            ("set x [gets stdin]\n\"[puts safe][eval $x]\"", "T100"),
        ] {
            let ws = of_code(source, D, code);
            assert!(
                ws.iter().any(|warning| warning.variable == "x"),
                "every inner command receives only its own substituted arguments: {source:?}: {ws:?}"
            );
        }
    }

    #[test]
    fn nested_script_sink_distinguishes_deferred_from_literal_arguments() {
        let ws = of_code("set x [gets stdin]\n[eval {$x}]", D, "T100");
        assert!(
            ws.iter().any(|warning| warning.variable == "x"),
            "eval reparses its braced argument as Tcl source: {ws:?}"
        );

        assert!(
            of_code("set x [gets stdin]\n[exec {$x}]", D, "T100").is_empty(),
            "exec receives a literal dollar spelling; it does not evaluate Tcl source"
        );

        let ws = of_code("set x [gets stdin]\n[interp eval child {$x}]", D, "T105");
        assert!(
            ws.iter().any(|warning| warning.variable == "x"),
            "interp eval reparses only its registry-declared body tail: {ws:?}"
        );
        assert!(
            of_code(
                "set x [gets stdin]\n[interp eval {$x} {set y safe}] $x",
                D,
                "T105",
            )
            .is_empty(),
            "a braced interpreter path and an outer argument are not child-interpreter code"
        );

        let ws = of_code("set x [gets stdin]\n[set y $x; eval {$y}]", D, "T100");
        assert!(
            ws.iter().any(|warning| warning.variable == "y"),
            "inner commands must propagate taint in Tcl evaluation order: {ws:?}"
        );
        assert!(
            of_code(
                "set x [gets stdin]\n[eval {set x safe; puts $x}]",
                D,
                "T100",
            )
            .is_empty(),
            "a script-local clean overwrite must cut off the enclosing tainted version"
        );
        assert!(
            of_code(
                "proc eval args {}\nset x [gets stdin]\n[eval $x]",
                D,
                "T100",
            )
            .is_empty(),
            "a nested call inherits the enclosing user-proc rebinding"
        );
        let ws = of_code(
            "interp alias {} myEval {} eval\nset x [gets stdin]\n[myEval $x]",
            D,
            "T100",
        );
        assert!(
            ws.iter().any(|warning| warning.variable == "x"),
            "a nested call inherits the enclosing static alias: {ws:?}"
        );
        for source in [
            "set command eval\nset x [gets stdin]\n[$command $x]",
            "set x [gets stdin]\nset script {$x}\n[eval $script]",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "nested analysis inherits phase-correct outer constants: {source}"
            );
        }
    }

    #[test]
    fn variable_command_head_uses_reaching_not_lexically_earlier_value() {
        assert!(
            of_code(
                "set command eval\nset command string\nset line [gets stdin]\n$command length $line",
                D,
                "T100",
            )
            .is_empty(),
            "a superseded eval spelling must not classify the later string call as a sink"
        );
    }

    #[test]
    fn variable_command_head_warns_when_any_reaching_literal_is_a_sink() {
        let ws = of_code(
            "if {$flag} {set command eval} else {set command puts}\n\
             set line [gets stdin]\n$command $line",
            D,
            "T100",
        );
        assert!(
            ws.iter().any(|warning| warning.variable == "line"),
            "a may-dispatch to eval is security-relevant even when another arm is safe: {ws:?}"
        );
    }

    #[test]
    fn equivalent_variable_command_head_spellings_emit_one_warning() {
        let ws = of_code(
            "if {$flag} {set command eval} else {set command ::eval}\n\
             set line [gets stdin]\n$command $line",
            D,
            "T100",
        );
        assert_eq!(
            ws.len(),
            1,
            "equivalent qualified sink spellings must not duplicate one use-site diagnostic: {ws:?}"
        );
        assert_eq!(ws[0].sink_command, "eval");
    }

    #[test]
    fn namespace_shadowed_variable_command_head_is_not_a_builtin_sink() {
        assert!(
            of_code(
                "proc ::ns::eval args {}\n\
                 proc ::ns::run {} {set command eval; set line [gets stdin]; $command $line}\n\
                 ::ns::run",
                D,
                "T100",
            )
            .is_empty(),
            "a namespace-local proc named eval must not inherit the builtin's sink semantics"
        );
    }

    #[test]
    fn namespace_shadowed_catch_cannot_hijack_nested_analysis_scaffolding() {
        let ws = of_code(
            "proc ::ns::catch args {}\n\
             proc ::ns::run {} {set x [gets stdin]; [catch {}; eval $x]}\n\
             ::ns::run",
            D,
            "T100",
        );
        assert!(
            ws.iter().any(|warning| warning.variable == "x"),
            "a user proc named catch must not suppress the real nested eval sink: {ws:?}"
        );
    }

    #[test]
    fn exec_taints() {
        // tclsh: `exec ls` returns subprocess stdout.
        let ws = of_code("set output [exec ls]\neval $output", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "output");
    }

    #[test]
    fn socket_taints() {
        let ws = of_code("set s [socket localhost 80]\neval $s", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn chan_read_taints() {
        let ws = of_code("set data [chan read $fd]\neval $data", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn chan_gets_taints() {
        let ws = of_code("set data [chan gets $fd]\neval $data", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn chan_configure_does_not_taint() {
        // tclsh: `chan configure $fd` returns channel options, not I/O content —
        // not a taint source.
        assert_eq!(
            codes("set x [chan configure $fd]\neval $x", D),
            Vec::<String>::new()
        );
    }
}

// ===========================================================================
// Registry object-instance sources.  Widget paths and object variables are
// commands at runtime, so the constructor's `object_class` / `creates_instance_at`
// metadata must carry the `TAINT_SOURCE` trait on `get` through that dispatch.
// These cases deliberately name no widget or getter in compiler code.
// ===========================================================================
mod object_instance_taint_sources {
    use super::*;

    #[test]
    fn literal_widget_path_get_taints() {
        let source = "ttk::entry .user\nset value [.user get]\neval $value";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "registry instance source should reach eval: {warnings:?}"
        );
    }

    #[test]
    fn instance_taint_sources_follow_the_resolved_widget_lifecycle() {
        // ttk::treeview `current` is a zero-argument taint source added in
        // Tk 9.1. The generic instance-invocation resolver must not replay
        // that source fact for a tcl9.0 profile merely because the class is
        // otherwise known there.
        let source = "ttk::treeview .tree\nset value [.tree current]\neval $value";
        for (dialect, expects_taint) in [("tcl9.0", false), ("tcl9.1", true)] {
            assert_eq!(
                !of_code(source, dialect, "T100").is_empty(),
                expects_taint,
                "ttk::treeview current taint source under {dialect}"
            );
        }
    }

    #[test]
    fn widget_handle_variable_get_taints() {
        let source = "set widget [ttk::entry .user]\nset value [$widget get]\neval $value";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "typed object handle source should reach eval: {warnings:?}"
        );
    }

    #[test]
    fn instance_source_flows_through_proc_summary() {
        let source = "proc user_value {} {\n    ttk::entry .user\n    return [.user get]\n}\nset value [user_value]\neval $value";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "instance source in a proc return should reach eval: {warnings:?}"
        );
    }

    #[test]
    fn top_level_widget_is_a_source_inside_callback_procedure() {
        let source = "ttk::entry .user\nproc submit {} {\n    set value [.user get]\n    eval $value\n}\nsubmit";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "top-level widget commands remain visible in callback procedures: {warnings:?}"
        );
    }

    #[test]
    fn text_dump_and_file_chooser_results_are_input_sources() {
        for source in [
            "text .editor\nset value [.editor dump 1.0 end]\neval $value",
            "set value [tk_getOpenFile]\neval $value",
            "set value [tk_chooseDirectory]\neval $value",
        ] {
            let warnings = of_code(source, D, "T100");
            assert!(
                warnings.iter().any(|warning| warning.variable == "value"),
                "user-selected or user-authored Tk value must be tainted: {warnings:?}"
            );
        }
    }

    #[test]
    fn ambiguous_instance_class_abstains() {
        let source = "ttk::entry .widget\nlistbox .widget\nset value [.widget get]\neval $value";
        assert!(
            of_code(source, D, "T100").is_empty(),
            "a receiver with incompatible registry classes must not be guessed"
        );
    }

    #[test]
    fn linked_entry_variable_is_user_input_and_show_is_not_a_sanitiser() {
        let source = "entry .password -show * -textvariable password\neval $password";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.variable == "password"),
            "password masking must not sanitize linked user input: {warnings:?}"
        );
    }

    #[test]
    fn deferred_validation_callback_preserves_link_and_instance_taint() {
        let source = "entry .password -textvariable password -validatecommand {return 1}\nset typed [.password get]\neval $password\neval $typed";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.variable == "password"),
            "the deferred callback must not hide the linked-variable definition: {warnings:?}"
        );
        assert!(
            warnings.iter().any(|warning| warning.variable == "typed"),
            "the deferred callback must not hide the constructor's instance class: {warnings:?}"
        );
    }

    #[test]
    fn validation_percent_values_taint_inline_and_list_prefix_callbacks() {
        for source in [
            "entry .password -validatecommand {set proposed %P; eval $proposed}",
            "entry .password -validatecommand [list eval %P]",
            "entry .password -validatecommand \"[list eval %P]\"",
            "entry .password -validatecommand \"eval %P\"",
            r"entry .password -validatecommand eval\ %P",
            "bind .password <Key> {set typed %A; eval $typed}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "registry-declared callback input must reach its sink: {source}"
            );
        }
    }

    #[test]
    fn validation_percent_value_in_command_position_is_dynamic_dispatch() {
        for source in [
            "entry .password -validatecommand {%P}",
            "entry .password -validatecommand {{*}%P}",
            "entry .password -validatecommand [list %P]",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "a callback-provided command head is tainted dynamic dispatch: {source}"
            );
        }
    }

    #[test]
    fn callback_replay_scaffold_ignores_user_catch_and_proc_commands() {
        for source in [
            "proc catch args {}\nentry .password -validatecommand {eval %P}",
            "proc proc args {}\nentry .password -validatecommand {eval %P}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "user commands named like replay plumbing must not suppress the callback sink: {source}"
            );
        }
    }

    #[test]
    fn resolved_widget_instance_callbacks_replay_registry_declared_inputs() {
        for source in [
            "canvas .canvas\n.canvas bind all <Key> {eval %A}",
            "entry .entry\n.entry configure -validatecommand {eval %P}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "a statically typed instance callback must use its registry descriptor: {source}"
            );
        }
    }

    #[test]
    fn dynamic_or_unknown_widget_receivers_do_not_invent_callback_sources() {
        for source in [
            ".unknown bind all <Key> {eval %A}",
            "canvas .canvas\nset receiver .canvas\n$receiver bind all <Key> {eval %A}",
        ] {
            assert!(
                of_code(source, D, "T100").is_empty(),
                "a non-singleton/literal receiver must not be guessed as a callback host: {source}"
            );
        }
    }

    #[test]
    fn validation_callback_percent_source_survives_braced_quoted_and_embedded_words() {
        for source in [
            // Tk substitutes `%P` before evaluating this script. Its braced
            // word is a user value, not a trusted literal for the later eval.
            "entry .password -validatecommand {eval {%P}}",
            "entry .password -validatecommand {eval \"prefix-%P\"}",
            "entry .password -validatecommand {set proposed {%P}; eval $proposed}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "Tk callback replacement must remain tainted across word quoting: {source}"
            );
        }
    }

    #[test]
    fn validation_callback_replay_preserves_later_word_substitutions() {
        for source in [
            "entry .password -validatecommand {set x %P; eval \"puts $x\"}",
            "entry .password -validatecommand {set x %P; eval prefix-$x}",
            "entry .password -validatecommand {set x %P; eval \"puts [set y $x]\"}",
            "entry .password -validatecommand {set command eval; $command %P}",
            r"entry .password -validatecommand {set x %P; e\166al $x}",
            r"entry .password -validatecommand {set x %P; eval \[eval\ \$x\]}",
            r"entry .password -validatecommand {set x %P; [e\166al $x]}",
            "entry .password -validatecommand {set x %P; [puts safe; eval $x]}",
            "entry .password -validatecommand {set command eval; set x %P; [$command $x]}",
            "entry .password -validatecommand {set x %P; [set y $x; eval {$y}]}",
            r"entry .password -validatecommand [list e\166al %P]",
            r"entry .password -validatecommand {set x %P; eval {eval\
 $x}}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "replay must preserve quoted, compound, and command substitutions: {source}"
            );
        }
    }

    #[test]
    fn validation_callback_replay_preserves_argument_expansion() {
        for source in [
            "entry .password -validatecommand {set x %P; {*}{{eval} safe} $x}",
            "entry .password -validatecommand {set x %P; set prefix [list eval safe]; {*}$prefix $x}",
            "entry .password -validatecommand [list {*}{{eval} safe} %P]",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "callback replay must retain the list-expanded eval prefix: {source}"
            );
        }
        assert!(
            of_code(
                "entry .password -validatecommand {set x %P; {*}{{puts} safe} $x}",
                D,
                "T100",
            )
            .is_empty(),
            "a list-expanded non-sink callback command must remain clean"
        );
    }

    #[test]
    fn validation_list_prefix_builder_uses_source_position_command_identity() {
        for source in [
            "interp alias {} make {} list\nentry .password -validatecommand [make eval %P]",
            "interp alias {} make {} list\nentry .password -validatecommand \"[make eval %P]\"",
            "rename list make\nentry .password -validatecommand [make eval %P]",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "an alias or rename of list constructs the same callback prefix: {source}"
            );
        }
        for source in [
            "proc list args {return {puts safe}}\nentry .password -validatecommand [list eval %P]",
            "interp alias {} make {} list puts\nentry .password -validatecommand [make eval %P]",
        ] {
            assert!(
                of_code(source, D, "T100").is_empty(),
                "a rebound builder or alias with prepended argv must not inherit bare list semantics: {source}"
            );
        }
    }

    #[test]
    fn validation_callback_replay_decodes_static_non_head_backslashes() {
        let source = r"entry .password -validatecommand {set x %P; eval puts\ \$x}";
        assert!(
            !of_code(source, D, "T101").is_empty(),
            "a backslash-built script argument must retain its decoded output-sink semantics"
        );
    }

    #[test]
    fn validation_list_prefix_preserves_a_quoted_command_name() {
        assert!(
            of_code(
                "entry .password -validatecommand [list {eval foo} %P]",
                D,
                "T100",
            )
            .is_empty(),
            "a list element naming one command with a space must not be replayed as builtin eval"
        );
    }

    #[test]
    fn validation_callback_replay_input_name_cannot_collide_with_program_variables() {
        let source =
            "entry .password -validatecommand {set __tcl_lsp_callback_input trusted; eval {%P}}";
        assert!(
            !of_code(source, D, "T100").is_empty(),
            "the replay source name must avoid a callback/program variable collision"
        );
    }

    #[test]
    fn validation_callback_seed_ignores_shadowed_scaffolding_commands() {
        let source = "proc gets {args} { return trusted }\n\
                      proc set {args} { return trusted }\n\
                      entry .password -validatecommand {eval {%P}}";
        assert!(
            !of_code(source, D, "T100").is_empty(),
            "Tk callback input is an external source even when gets/set are shadowed"
        );
    }

    #[test]
    fn validation_callback_direct_input_diagnostic_hides_synthetic_state() {
        let source = "entry .password -validatecommand {eval %P}";
        let warnings = of_code(source, D, "T100");
        let warning = warnings
            .iter()
            .find(|warning| warning.variable == "Tk callback input %P")
            .expect("direct Tk input warning: {warnings:?}");
        assert!(
            warning.message.contains("Tk callback input %P"),
            "the user-facing message must identify the Tk source: {warning:?}"
        );
        assert!(
            !warning.message.contains("__tcl_lsp_callback"),
            "synthetic replay state must not leak to the user: {warning:?}"
        );
        assert_eq!(
            warning.span.start(),
            u32::try_from(source.find("{eval %P}").expect("callback span"))
                .expect("test source fits in u32"),
            "the direct callback sink maps back to its registration"
        );
        assert!(
            warning.replacement.is_none() && warning.fixes.is_empty(),
            "synthetic replay diagnostics cannot carry synthetic-source edits: {warning:?}"
        );
    }

    #[test]
    fn callback_replays_isolate_abnormal_completion_and_handler_locals() {
        for first in ["return %P", "error %P"] {
            let source = format!(
                "entry .first -validatecommand {{{first}}}\n\
                 entry .second -validatecommand {{eval %P}}"
            );
            let second_start = u32::try_from(source.rfind("{eval %P}").expect("second callback"))
                .expect("test source fits in u32");
            assert!(
                of_code(&source, D, "T100")
                    .iter()
                    .any(|warning| warning.span.start() == second_start),
                "an abnormal first callback cannot make its sibling unreachable: {source}"
            );
        }

        let clean_then_tainted = "entry .first -validatecommand {set x safe; return %P}\n\
                                  entry .second -validatecommand {set x %P; eval $x}";
        assert!(
            !of_code(clean_then_tainted, D, "T100").is_empty(),
            "a prior callback's clean local cannot kill the later handler's source"
        );

        let tainted_then_clean = "entry .first -validatecommand {set x %P}\n\
                                  entry .second -validatecommand {set unused %P; eval $x}";
        assert!(
            of_code(tainted_then_clean, D, "T100").is_empty(),
            "a prior callback's local cannot invent a warning in a sibling handler"
        );
    }

    #[test]
    fn validation_list_prefix_taints_a_named_proc_parameter() {
        let source = "proc validate {proposed} { eval $proposed }\nentry .password -validatecommand [list validate %P]";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.variable == "proposed"),
            "a list-built callback prefix must seed its procedure parameter: {warnings:?}"
        );
        let sink_start = u32::try_from(source.find("$proposed").expect("procedure sink variable"))
            .expect("test source fits in u32");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.variable == "proposed" && warning.span.start() == sink_start),
            "a proc-prefix warning belongs on the real sink, not the registration: {warnings:?}"
        );
    }

    #[test]
    fn callback_framework_metadata_and_escaped_percent_stay_clean() {
        for source in [
            "entry .password -validatecommand {eval %W}",
            "entry .password -validatecommand {eval %%P}",
            "bind .password <Key> {eval %W}",
            "bind .password <Key> {eval %K}",
            "entry .password -validatecommand [format {eval %P}]",
        ] {
            assert!(
                of_code(source, D, "T100").is_empty(),
                "metadata, escaped markers, and dynamic callback builders stay untainted: {source}"
            );
        }
    }

    #[test]
    fn user_controlled_non_text_widget_variable_is_tainted() {
        let source = "checkbutton .choice -variable choice\neval $choice";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "choice"),
            "user-controlled linked widget state must be tainted: {warnings:?}"
        );
    }

    #[test]
    fn display_only_textvariable_and_widget_handle_stay_clean() {
        assert!(
            of_code(
                "set safe fixed\nlabel .display -textvariable safe\neval $safe",
                D,
                "T100"
            )
            .is_empty(),
            "a display-only textvariable is not a user-input source"
        );
        assert!(
            of_code("set widget [entry .user]\neval $widget", D, "T100").is_empty(),
            "a widget constructor's trusted handle is not user input"
        );
    }

    #[test]
    fn mixed_widget_links_taint_only_the_user_editable_variable() {
        let source = "set caption fixed\ncheckbutton .choice -textvariable caption -variable choice\neval $caption\neval $choice";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "choice"),
            "the selection variable is user controlled: {warnings:?}"
        );
        assert!(
            warnings.iter().all(|warning| warning.variable != "caption"),
            "the display-only textvariable must stay clean: {warnings:?}"
        );
    }

    #[test]
    fn instance_configure_adds_a_phase_correct_external_input_definition() {
        let source = "entry .user\neval $value\n.user configure -textvariable value\neval $value";
        let warnings = of_code(source, D, "T100");
        assert_eq!(
            warnings
                .iter()
                .filter(|warning| warning.variable == "value")
                .count(),
            1,
            "only the use after the registry-typed configure link is tainted: {warnings:?}"
        );

        let source = "set widget [ttk::entry .user]\n$widget configure -textvariable value\nset copy $value\neval $copy";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "copy"),
            "a typed variable receiver's configured input link must propagate: {warnings:?}"
        );
    }

    #[test]
    fn display_only_instance_configure_does_not_taint_its_variable() {
        let source = "label .caption\n.caption configure -textvariable caption\neval $caption";
        assert!(
            of_code(source, D, "T100").is_empty(),
            "a display-only class option must not become an input source"
        );
    }

    #[test]
    fn argument_sensitive_widget_getters_taint_only_user_state_forms() {
        for source in [
            "scale .s\nset user [.s get]\nset derived [.s get 10 20]\neval $user\neval $derived",
            "ttk::toggleswitch .s\nset user [.s get]\nset derived [.s get min]\neval $user\neval $derived",
            "ttk::toggleswitch .s\nset user [.s switchstate]\nset derived [.s switchstate true]\neval $user\neval $derived",
            "ttk::toggleswitch .s\nset user [.s xcoord]\nset derived [.s xcoord 0.5]\neval $user\neval $derived",
        ] {
            let warnings = of_code(source, D, "T100");
            assert!(
                warnings.iter().any(|warning| warning.variable == "user"),
                "the zero-argument getter reads user state: {warnings:?}"
            );
            assert!(
                !warnings.iter().any(|warning| warning.variable == "derived"),
                "the explicit coordinate/selector getter is derived, not input: {warnings:?}"
            );
        }
    }

    #[test]
    fn overloaded_treeview_and_notebook_getters_only_taint_zero_arg_queries() {
        for (dialect, query, setter) in [
            (
                D,
                "ttk::combobox .combo\nset user [.combo current]\neval $user",
                "ttk::combobox .combo\nset value [.combo current 0]\neval $value",
            ),
            (
                D,
                "ttk::treeview .tree\nset user [.tree selection]\neval $user",
                "ttk::treeview .tree\nset value [.tree selection set item]\neval $value",
            ),
            (
                "tcl9.0",
                "ttk::treeview .tree\nset user [.tree cellselection]\neval $user",
                "ttk::treeview .tree\nset value [.tree cellselection set item]\neval $value",
            ),
            (
                "tcl9.1",
                "ttk::treeview .tree\nset user [.tree current]\neval $user",
                "ttk::treeview .tree\nset value [.tree current ignored]\neval $value",
            ),
            (
                D,
                "ttk::treeview .tree\nset user [.tree focus]\neval $user",
                "ttk::treeview .tree\nset value [.tree focus item]\neval $value",
            ),
            (
                "tcl9.1",
                "ttk::treeview .tree\nset user [.tree cellfocus]\neval $user",
                "ttk::treeview .tree\nset value [.tree cellfocus 0,0]\neval $value",
            ),
            (
                D,
                "ttk::notebook .book\nset user [.book select]\neval $user",
                "ttk::notebook .book\nset value [.book select .tab]\neval $value",
            ),
        ] {
            assert!(
                of_code(query, dialect, "T100")
                    .iter()
                    .any(|warning| warning.variable == "user"),
                "zero-argument Tk UI query must be a source under {dialect}: {query}"
            );
            assert!(
                of_code(setter, dialect, "T100").is_empty(),
                "setter/argument overload must not be a source under {dialect}: {setter}"
            );
        }
    }

    #[test]
    fn pointer_position_queries_are_user_input_sources() {
        for query in [
            "winfo pointerx .",
            "winfo pointery .",
            "winfo pointerxy .",
            "winfo containing 10 20",
            "winfo containing -displayof . 10 20",
        ] {
            let source = format!("set user [{query}]\neval $user");
            let warnings = of_code(&source, D, "T100");
            assert!(
                warnings.iter().any(|warning| warning.variable == "user"),
                "pointer-dependent query must be a source: {query}; {warnings:?}"
            );
        }
    }

    #[test]
    fn later_constructor_does_not_retroactively_type_an_earlier_receiver() {
        assert!(
            of_code(
                "set value [.future get]\nttk::entry .future\neval $value",
                D,
                "T100"
            )
            .is_empty(),
            "receiver typing must respect source order"
        );
    }

    #[test]
    fn destroy_kills_local_instance_class_and_recreate_restores_it() {
        assert!(
            of_code(
                "entry .user\ndestroy .user\nset value [.user get]\neval $value",
                D,
                "T100"
            )
            .is_empty(),
            "a registry teardown must remove the destroyed widget from local receiver facts"
        );
        assert!(
            of_code(
                "entry .user\ndestroy .user\nentry .user\nset value [.user get]\neval $value",
                D,
                "T100"
            )
            .iter()
            .any(|warning| warning.variable == "value"),
            "a later constructor must recreate the receiver class after teardown"
        );

        assert!(
            of_code(
                "frame .panel\nentry .panel.user\ndestroy .\nset value [.panel.user get]\neval $value",
                D,
                "T100"
            )
            .is_empty(),
            "destroying the Tk root must kill every descendant receiver fact"
        );
    }

    #[test]
    fn rename_moves_local_and_global_instance_class_facts() {
        let local = "entry .user\nrename .user .renamed\nset value [.renamed get]\neval $value";
        assert!(
            of_code(local, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "registry command-table rename must move a local widget class"
        );

        let global = "entry .user\nrename .user .renamed\nproc read_value {} {\n    set value [.renamed get]\n    eval $value\n}\nread_value";
        assert!(
            of_code(global, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "registry command-table rename must move global callback class facts"
        );
    }

    #[test]
    fn global_destroy_kills_callback_instance_class_facts() {
        let source = "entry .user\ndestroy .user\nproc read_value {} {\n    set value [.user get]\n    eval $value\n}\nread_value";
        assert!(
            of_code(source, D, "T100").is_empty(),
            "a top-level registry teardown must not leave a stale global callback receiver"
        );
    }

    #[test]
    fn instance_method_abbreviations_resolve_only_when_unique() {
        let warnings = of_code(
            "entry .user\nset value [.user g]\neval $value\n.user co -textvariable linked\neval $linked",
            D,
            "T100",
        );
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "the unique `get` prefix must retain source semantics: {warnings:?}"
        );
        assert!(
            warnings.iter().any(|warning| warning.variable == "linked"),
            "the unique `configure` prefix must retain option-write semantics: {warnings:?}"
        );

        assert!(
            of_code(
                "entry .user\n.user c -textvariable linked\neval $linked",
                D,
                "T100"
            )
            .is_empty(),
            "`c` is ambiguous between cget/configure and must abstain"
        );
    }

    #[test]
    fn literal_global_receiver_configure_taints_across_procedures() {
        let source = "entry .user\nproc link_input {} {\n    .user configure -textvariable ::value\n}\nlink_input\neval $::value";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "::value"),
            "a callee's proven widget link must define and taint the caller's global: {warnings:?}"
        );

        let nested = "entry .user\nproc inner {} {\n    .user configure -textvariable ::value\n}\nproc outer {} { inner }\nouter\neval $::value";
        let warnings = of_code(nested, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "::value"),
            "instance-option global writes must close transitively over calls: {warnings:?}"
        );
    }

    #[test]
    fn tk_unqualified_link_targets_global_not_proc_local_state() {
        let false_positive = "package require Tk\nproc p {} {\n    set value safe\n    entry .e -textvariable value\n    eval $value\n}\np";
        assert!(
            of_code(false_positive, D, "T100").is_empty(),
            "Tk's unqualified link is ::value; it must not taint proc-local value"
        );

        let false_negative = "package require Tk\nproc p {} {\n    entry .e -textvariable value\n}\np\neval $::value";
        let warnings = of_code(false_negative, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "::value"),
            "Tk's unqualified link must taint the documented global ::value: {warnings:?}"
        );
    }

    #[test]
    fn constructor_and_configure_global_links_close_over_proc_calls() {
        for source in [
            "proc inner {} { entry .user -textvariable value }\nproc outer {} { inner }\nouter\neval $::value",
            "entry .user\nproc inner {} { .user configure -textvariable value }\nproc outer {} { inner }\nouter\neval $::value",
            "proc inner {} { entry .user -textvariable ::value }\ninner\neval $::value",
        ] {
            let warnings = of_code(source, D, "T100");
            assert!(
                warnings.iter().any(|warning| warning.variable == "::value"),
                "registry-global Tk link must propagate through callers: {warnings:?}"
            );
        }
    }

    #[test]
    fn dynamic_parameter_receiver_configure_abstains() {
        let source = "entry .user\nproc link_input {widget} {\n    $widget configure -textvariable ::value\n}\nlink_input .user\neval $::value";
        assert!(
            of_code(source, D, "T100").is_empty(),
            "a parameter receiver has no uniform class proof and must not be guessed"
        );
    }

    #[test]
    fn eager_call_and_conditional_constructor_do_not_create_global_receiver_facts() {
        let called_too_early = "proc link_input {} {\n    .user configure -textvariable ::value\n}\nlink_input\nentry .user\neval $::value";
        assert!(
            of_code(called_too_early, D, "T100").is_empty(),
            "a constructor after an eager call cannot type that invocation"
        );

        let conditional = "if {$flag} { entry .user }\nproc link_input {} {\n    .user configure -textvariable ::value\n}\nlink_input\neval $::value";
        assert!(
            of_code(conditional, D, "T100").is_empty(),
            "a conditional constructor does not prove a receiver exists"
        );

        let conditional_rebind = "entry .user\nif {$flag} { label .user }\nproc read_input {} {\n    set value [.user get]\n    eval $value\n}\nread_input";
        assert!(
            of_code(conditional_rebind, D, "T100").is_empty(),
            "a possible later constructor withdraws an earlier receiver proof"
        );
    }

    #[test]
    fn receiver_rebinding_and_branch_joins_use_reaching_classes() {
        assert!(
            of_code(
                "set widget [entry .user]\nset widget [label .display]\nset value [$widget get]\neval $value",
                D,
                "T100"
            )
            .is_empty(),
            "a later non-source receiver binding must kill the earlier entry class"
        );

        let rebound = "set widget [label .display]\nset widget [entry .user]\nset value [$widget get]\neval $value";
        assert!(
            of_code(rebound, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "the currently reaching entry binding must be used"
        );

        let same_class_join = "if {$flag} {\n    set widget [entry .one]\n} else {\n    set widget [entry .two]\n}\nset value [$widget get]\neval $value";
        assert!(
            of_code(same_class_join, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "a join whose paths agree on class remains typed"
        );

        let mixed_join = "if {$flag} {\n    set widget [entry .one]\n} else {\n    set widget [label .two]\n}\nset value [$widget get]\neval $value";
        assert!(
            of_code(mixed_join, D, "T100").is_empty(),
            "a join with multiple possible classes must abstain"
        );

        let global_rebound = "entry .shared\nlabel .shared\nproc use_value {} {\n    set value [.shared get]\n    eval $value\n}\nuse_value";
        assert!(
            of_code(global_rebound, D, "T100").is_empty(),
            "the last unconditional constructor determines a global literal receiver"
        );

        let global_rebound_to_source = "label .shared\nentry .shared\nproc use_value {} {\n    set value [.shared get]\n    eval $value\n}\nuse_value";
        assert!(
            of_code(global_rebound_to_source, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "a global literal receiver rebound to an entry remains a source"
        );
    }
}

// ===========================================================================
// iRules I/O getters taint their results.
// f5-dialect: HTTP::*, TCP::*, IP::* are not core-tclsh commands.
// ===========================================================================
mod irules_taint_sources {
    use super::*;

    #[test]
    fn http_payload_taints() {
        // f5-dialect: HTTP::payload returns the request body.
        let ws = of_code("set body [HTTP::payload]\neval $body", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "body");
    }

    #[test]
    fn http_header_taints() {
        // f5-dialect.
        let ws = of_code("set hdr [HTTP::header \"Host\"]\neval $hdr", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn http_uri_taints() {
        // f5-dialect.
        let ws = of_code("set uri [HTTP::uri]\neval $uri", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn tcp_payload_taints() {
        // f5-dialect.
        let ws = of_code("set data [TCP::payload]\neval $data", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn ip_client_addr_taints_with_colour() {
        // f5-dialect: IP addresses are tainted but carry the IP_ADDRESS colour.
        let ws = of_code("set addr [IP::client_addr]\neval $addr", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "addr");
    }
}

// ===========================================================================
// Taint flows through assignments and interpolation.
// ===========================================================================
mod taint_propagation {
    use super::*;

    #[test]
    fn variable_copy_propagates() {
        let ws = of_code("set x [read $fd]\nset y $x\neval $y", D, "T100");
        assert!(ws.iter().any(|w| w.variable == "y"));
    }

    #[test]
    fn string_interpolation_propagates() {
        let ws = of_code(
            "set x [read $fd]\nset y \"prefix${x}suffix\"\neval $y",
            D,
            "T100",
        );
        assert!(ws.iter().any(|w| w.variable == "y"));
    }

    #[test]
    fn command_subst_concat_propagates() {
        let ws = of_code("set x [read $fd]/suffix\neval $x", D, "T100");
        assert!(ws.iter().any(|w| w.variable == "x"));
    }

    #[test]
    fn sanitiser_blocks_taint() {
        // tclsh: `string length $x` returns an int (`11` for an 11-char value) —
        // a fixed numeric return cannot carry taint, so the result is clean.
        assert_eq!(
            codes("set x [read $fd]\nset n [string length $x]\nexpr $n", D),
            Vec::<String>::new()
        );
    }

    #[test]
    fn llength_sanitises() {
        // tclsh: `llength` returns an int (element count) — sanitises taint.
        assert_eq!(
            codes("set x [read $fd]\nset n [llength $x]\nexpr $n", D),
            Vec::<String>::new()
        );
    }

    #[test]
    fn taint_through_if_branch() {
        let source =
            "set x [read $fd]\nif {1} {\n    set y $x\n} else {\n    set y \"safe\"\n}\neval $y\n";
        let ws = of_code(source, D, "T100");
        assert!(ws.iter().any(|w| w.variable == "y"));
    }
}

// ===========================================================================
// Proc summaries propagate taint across calls.
// ===========================================================================
mod interprocedural_taint {
    use super::*;

    #[test]
    fn helper_sanitiser_suppresses_taint() {
        // tclsh: the helper returns `string length` (an int) → clean result.
        let source = "proc safe_len {x} { return [string length $x] }\nset raw [read $fd]\nset n [safe_len $raw]\nexpr $n\n";
        assert_eq!(codes(source, D), Vec::<String>::new());
    }

    #[test]
    fn tainted_return_from_helper_reaches_sink() {
        // f5-dialect: helper returns HTTP::payload (tainted) → flows to eval.
        let source = "proc payload {} { return [HTTP::payload] }\nset data [payload]\neval $data\n";
        let ws = of_code(source, D, "T100");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.variable == "data"));
    }

    #[test]
    fn tainted_argument_into_sinking_helper_warns() {
        let source = "proc do_eval {x} { eval $x }\nset data [read $fd]\ndo_eval $data\n";
        let ws = of_code(source, D, "T100");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn tainted_variadic_args_into_sinking_helper_warns() {
        // `proc log {args} { puts $args }` packs every trailing actual
        // argument into the `args` list parameter — a tainted actual must
        // still be traced (via the interprocedural entry-taint solve's
        // `args`-packing rule) into the sink inside the callee body.
        let source = "proc log {args} { puts $args }\nset data [read $fd]\nlog $data\n";
        let ws = of_code(source, D, "T101");
        assert!(
            !ws.is_empty(),
            "expected T101 for tainted args-packed data, got {ws:?}"
        );
    }
}

// ===========================================================================
// Dangerous sinks (T100) — tainted data in eval/expr/exec/uplevel/subst.
//
// tclsh: each of these re-parses/executes its argument, so a tainted value is a
// code-execution vector. (eval/uplevel/subst/exec run the value; unbraced expr
// re-parses it.)
// ===========================================================================
mod dangerous_sinks {
    use super::*;

    #[test]
    fn eval_sink() {
        let ws = of_code("set x [read $fd]\neval $x", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].sink_command, "eval");
    }

    #[test]
    fn expr_sink() {
        let ws = of_code("set x [read $fd]\nexpr $x", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn exec_sink() {
        let ws = of_code("set x [read $fd]\nexec $x", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].sink_command, "exec");
    }

    #[test]
    fn uplevel_sink() {
        let ws = of_code("set x [read $fd]\nuplevel $x", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn subst_sink() {
        let ws = of_code("set x [read $fd]\nsubst $x", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn coroprobe_command_prefix_sink() {
        // `coroprobe coroName command ?arg ...?` evaluates `command` in a
        // suspended coroutine's frame right now — it carries
        // `Traits::EVALUATES_CODE` but (unlike `eval`/`exec`/`uplevel`) not
        // `Traits::TAINT_SINK`, so the registry-driven sink classification
        // must still route it to T100 via the `EVALUATES_CODE` trait rather
        // than silently losing it now that sink classification is
        // registry-driven instead of hardcoded per-trait checks in the
        // compiler.
        let ws = of_code(
            "set cmd [read $fd]\ncoroprobe myCoro $cmd",
            "tcl9.0",
            "T100",
        );
        assert!(!ws.is_empty(), "expected T100 for coroprobe, got none");
    }

    #[test]
    fn coroinject_command_prefix_sink() {
        let ws = of_code(
            "set cmd [read $fd]\ncoroinject myCoro $cmd",
            "tcl9.0",
            "T100",
        );
        assert!(!ws.is_empty(), "expected T100 for coroinject, got none");
    }
}

// ===========================================================================
// The T100 message names the sink and the variable.
// ===========================================================================
mod warning_messages {
    use super::*;

    #[test]
    fn taint_warning_message() {
        let ws = of_code("set x [read $fd]\neval $x", D, "T100");
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("eval"));
        assert!(ws[0].message.contains("$x") || ws[0].message.contains('x'));
    }
}

// ===========================================================================
// Output sinks (T101) — tainted data flowing into `puts`.
//
// tclsh: `puts ?-nonewline? ?channelId? string` — only the trailing content
// word is output content; a tainted channel id is a destination handle and must
// not flag T101.
// ===========================================================================
mod output_sinks {
    use super::*;

    #[test]
    fn puts_tainted_data_span_is_tight_around_argument() {
        // The diagnostic must underline just the tainted `$x` argument word,
        // not the whole `puts $x` statement — precise highlighting is what
        // lets the developer see exactly which value is the problem.
        let src = "set x [read $fd]\nputs $x";
        let ws = of_code(src, D, "T101");
        assert_eq!(ws.len(), 1);
        let want_start = u32::try_from(src.find("$x").unwrap()).unwrap();
        let want_end = want_start + 2; // "$x" is 2 bytes.
        assert_eq!(
            (ws[0].span.start(), ws[0].span.end()),
            (want_start, want_end),
            "expected the span to cover only `$x`, not the whole statement"
        );
    }

    #[test]
    fn puts_tainted_data() {
        let ws = of_code("set x [read $fd]\nputs $x", D, "T101");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "x");
        assert!(ws[0].sink_command.contains("puts"));
    }

    #[test]
    fn puts_literal_clean() {
        assert!(of_code("puts \"hello world\"", D, "T101").is_empty());
    }

    #[test]
    fn puts_sanitised_clean() {
        assert!(
            of_code(
                "set x [read $fd]\nset n [string length $x]\nputs $n",
                D,
                "T101",
            )
            .is_empty()
        );
    }

    #[test]
    fn puts_tainted_channel_position_silent() {
        // tclsh: in `puts -nonewline $chan {hello}`, $chan is the destination
        // channel, not injectable content (tcllib imap4.tcl idiom). No T101.
        assert!(
            of_code(
                "set chan [read $fd]\nputs -nonewline $chan {hello}",
                D,
                "T101",
            )
            .is_empty()
        );
    }

    #[test]
    fn t100_tainted_inside_cmd_subst_in_expr_silent() {
        // tclsh: in `expr {([string length $data] / 8) * 8}`, $data is consumed
        // by the inner `string length`; expr sees only the int result, not the
        // original $data. No injection risk → T100 must not fire on $data.
        assert!(
            of_code(
                "set data [read $fd]\nset n [expr {([string length $data] / 8) * 8}]",
                D,
                "T100",
            )
            .is_empty()
        );
    }

    #[test]
    fn t100_tainted_direct_expr_operand_still_fires() {
        // Control: $data as a DIRECT expr operand IS the injection vector.
        let ws = of_code("set data [read $fd]\nset v [expr {$data + 1}]", D, "T100");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "data");
    }

    #[test]
    fn t100_tainted_expr_func_arg_still_fires() {
        // `abs($data)` — a math-func arg is a direct expr operand → T100 fires.
        let ws = of_code("set data [read $fd]\nset v [expr {abs($data)}]", D, "T100");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn puts_tainted_output_alongside_tainted_chan() {
        // Control: when BOTH channel and content are tainted, only the content
        // word flags T101.
        let ws = of_code(
            "set chan [read $fd]\nset msg [read $fd]\nputs -nonewline $chan $msg",
            D,
            "T101",
        );
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "msg");
    }

    #[test]
    fn puts_interpolation_propagates() {
        let ws = of_code("set x [read $fd]\nputs \"data: $x\"", D, "T101");
        assert!(!ws.is_empty());
    }

    #[test]
    fn puts_nonewline_alone_is_content_not_channel() {
        // `puts -nonewline $x` has only ONE positional arg after the flag,
        // so per Tcl semantics `$x` is the content written to stdout, not a
        // channel — the `-nonewline`-shifted `ArgRole::Channel` position
        // (declared dynamically since the channel slot moves with the
        // optional flag) must not exempt it.
        let ws = of_code("set x [read $fd]\nputs -nonewline $x", D, "T101");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "x");
    }

    #[test]
    fn puts_via_interp_alias_still_fires() {
        // `interp alias {} myputs {} puts` makes `myputs` a genuine
        // current-interpreter alias for the real `puts` builtin. The sink
        // classification must resolve the call through the lowerer's
        // `canonical_command` (populated for a resolved alias) rather than
        // the literal source spelling `"myputs"`, which isn't itself a
        // registered command and would otherwise silently miss the sink.
        let ws = of_code(
            "interp alias {} myputs {} puts\nset x [read $fd]\nmyputs $x",
            D,
            "T101",
        );
        assert_eq!(ws.len(), 1, "expected T101 via interp alias, got {ws:?}");
        assert_eq!(ws[0].variable, "x");
    }

    #[test]
    fn puts_inside_tcloo_method_body_still_fires() {
        // TclOO method bodies are analysable functions like any proc — the
        // sink check must not be limited to top-level / plain-proc bodies.
        let ws = of_code(
            "oo::class create Foo {\n\
             method bar {} {\n\
             set x [read $fd]\n\
             puts $x\n\
             }\n\
             }",
            D,
            "T101",
        );
        assert_eq!(
            ws.len(),
            1,
            "expected T101 inside TclOO method body, got {ws:?}"
        );
        assert_eq!(ws[0].variable, "x");
    }

    #[test]
    fn puts_inside_namespace_eval_body_still_fires() {
        // `namespace eval` registers its block as a synthetic body unit
        // (`Module::body_units`) — a fresh-frame body outside any proc —
        // which must be covered by the same sink check as the top level.
        let ws = of_code(
            "namespace eval ::myns {\n\
             set x [read $fd]\n\
             puts $x\n\
             }",
            D,
            "T101",
        );
        assert_eq!(
            ws.len(),
            1,
            "expected T101 inside namespace eval body, got {ws:?}"
        );
        assert_eq!(ws[0].variable, "x");
    }
}

// ===========================================================================
// Option injection (T102) — tainted value in option position, no `--`.
//
// tclsh: option-bearing commands (`regexp`, `glob`, …) parse a leading `-` as a
// switch, so a tainted value whose runtime content could start with `-` is an
// option-injection vector unless a `--` terminator ends switch scanning. A
// later *literal* positional also ends scanning (a tainted *subject* after a
// literal pattern is safe). Verified: `regexp -- -foo bar` → `0` (-- ends scan).
// ===========================================================================
mod option_injection {
    use super::*;

    #[test]
    fn regexp_tainted_without_terminator() {
        let ws = of_code("set x [read $fd]\nregexp $x test", D, "T102");
        assert!(!ws.is_empty());
        assert!(ws[0].sink_command.contains("regexp"));
    }

    #[test]
    fn regexp_tainted_with_terminator() {
        // tclsh: `--` ends switch scanning, so $x can only be the pattern.
        assert!(of_code("set x [read $fd]\nregexp -- $x test", D, "T102").is_empty());
    }

    #[test]
    fn glob_tainted_without_terminator() {
        let ws = of_code("set x [read $fd]\nglob $x", D, "T102");
        assert!(!ws.is_empty());
    }

    #[test]
    fn glob_tainted_with_terminator() {
        assert!(of_code("set x [read $fd]\nglob -- $x", D, "T102").is_empty());
    }

    #[test]
    fn literal_no_warning() {
        assert!(of_code("regexp {^/api} test", D, "T102").is_empty());
    }

    #[test]
    fn untainted_var_no_warning() {
        assert!(of_code("set x \"pattern\"\nregexp $x test", D, "T102").is_empty());
    }

    #[test]
    fn tainted_subject_after_literal_pattern_no_warning() {
        // tclsh: the literal pattern `{version ([0-9]+)}` is a definite
        // positional that ends switch scanning, so the tainted subject `$x` in a
        // later slot can't be misread as a switch. No T102.
        assert!(
            of_code(
                "set x [read $fd]\nregexp {version ([0-9]+)} $x -> m",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn braced_exec_argument_is_not_option_injection() {
        assert!(
            of_code("set x [read $fd]\nexec {$x}", D, "T102").is_empty(),
            "a braced dollar spelling is literal and cannot become an exec option"
        );
    }

    #[test]
    fn regsub_tainted_subject_after_literal_pattern_no_warning() {
        // `regsub -all {literal} $subject {}` — $subject is positional after the
        // literal pattern, not a switch position.
        assert!(
            of_code(
                "set x [read $fd]\nset y [regsub -all {/\\*.*?\\*/} $x {}]",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn unset_literal_name_no_warning() {
        // tclsh: `unset name` takes a literal variable name that cannot start
        // with `-`, so even a tainted var by that name is not option-injectable.
        assert!(of_code("set thelongname [read $fd]\nunset thelongname", D, "T102").is_empty());
    }

    #[test]
    fn regexp_tainted_pattern_still_warns() {
        // A tainted *pattern* (leading substitution, could expand to `-x`)
        // remains a T102 candidate even when a later positional is literal.
        let ws = of_code("set x [read $fd]\nregexp $x hello", D, "T102");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "x");
        assert!(ws[0].sink_command.contains("regexp"));
    }

    #[test]
    fn switch_after_dash_option_no_warning_for_literal_arg() {
        // `regexp -nocase {ab} $x` — after `-nocase` the literal pattern ends
        // scanning, so the tainted subject is safe.
        assert!(of_code("set x [read $fd]\nregexp -nocase {ab} $x", D, "T102").is_empty());
    }
}

// ===========================================================================
// IRULE3001 (HTTP::respond body) / IRULE3002
// (HTTP::header|cookie insert|replace). f5-dialect; gated on the iRules dialect.
// ===========================================================================
mod irules_output_sinks {
    use super::*;

    #[test]
    fn http_respond_tainted_body() {
        // f5-dialect.
        let ws = of_code(
            "set data [HTTP::payload]\nHTTP::respond 200 content $data",
            IR,
            "IRULE3001",
        );
        assert!(!ws.is_empty());
        assert!(ws[0].sink_command.contains("HTTP::respond"));
    }

    #[test]
    fn http_respond_literal_clean() {
        assert!(of_code("HTTP::respond 200 content \"ok\"", IR, "IRULE3001").is_empty());
    }

    #[test]
    fn http_header_insert_tainted() {
        // f5-dialect.
        let ws = of_code(
            "set val [HTTP::header Host]\nHTTP::header insert X-Foo $val",
            IR,
            "IRULE3002",
        );
        assert!(!ws.is_empty());
        assert!(ws[0].sink_command.to_lowercase().contains("header"));
    }

    #[test]
    fn http_header_remove_clean() {
        // `remove` is not an insert/replace value sink.
        assert!(of_code("HTTP::header remove X-Bad", IR, "IRULE3002").is_empty());
    }

    #[test]
    fn http_cookie_insert_tainted() {
        // f5-dialect.
        let ws = of_code(
            "set val [HTTP::cookie value session]\nHTTP::cookie insert name \"track\" value $val",
            IR,
            "IRULE3002",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn not_in_tcl86_dialect() {
        // Under plain tcl8.6 the HTTP::respond sink is not classified → no
        // IRULE3001 even though `read` taints `$data`.
        assert!(
            of_code(
                "set data [read $fd]\nHTTP::respond 200 content $data",
                D,
                "IRULE3001",
            )
            .is_empty()
        );
    }
}

// ===========================================================================
// Log injection (IRULE3003) — tainted data in `log`. f5-dialect.
// ===========================================================================
mod log_injection {
    use super::*;

    #[test]
    fn log_tainted_data() {
        // f5-dialect.
        let ws = of_code("set x [HTTP::uri]\nlog local0. $x", IR, "IRULE3003");
        assert!(!ws.is_empty());
        assert!(ws[0].sink_command.contains("log"));
    }

    #[test]
    fn log_literal_clean() {
        assert!(of_code("log local0. \"static message\"", IR, "IRULE3003").is_empty());
    }

    #[test]
    fn log_not_in_tcl86() {
        assert!(of_code("set x [read $fd]\nlog local0. $x", D, "IRULE3003").is_empty());
    }
}

// ===========================================================================
// PATH_PREFIXED / NON_DASH colours propagate through copies
// and interpolation, keeping later T102 suppressed. f5-dialect sources.
// ===========================================================================
mod taint_colours {
    use super::*;

    #[test]
    fn http_uri_getter_has_path_colour() {
        // T100 still fires (dangerous sink regardless of colour).
        let ws = of_code("set uri [HTTP::uri]\neval $uri", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "uri");
    }

    #[test]
    fn http_path_getter_has_path_colour() {
        let ws = of_code("set p [HTTP::path]\neval $p", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "p");
    }

    #[test]
    fn colour_propagates_through_copy() {
        let ws = of_code("set x [HTTP::uri]\nset y $x\neval $y", D, "T100");
        assert!(ws.iter().any(|w| w.variable == "y"));
    }

    #[test]
    fn non_dash_literal_prefix_stays_t102_safe() {
        // `"prefix${uri}"` no longer starts with `/` but still cannot start with
        // `-` → NON_DASH_PREFIXED, T102 safe.
        assert!(
            of_code(
                "set uri [HTTP::uri]\nset z \"prefix${uri}\"\nregexp $z test",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn colour_kept_when_dynamic_leading_piece_is_path_prefixed() {
        // `${uri}/suffix` keeps PATH_PREFIXED (the leading dynamic
        // piece is the path-prefixed uri), so T102 stays suppressed.
        // FIXED (lattice-join identity): `word_taint` seeds the interpolation
        // with `clean()` and folds `join` over each piece; with clean now the
        // join identity (not an annihilator) the first join keeps PATH_PREFIXED,
        // so the T102_SAFE colour survives and T102 stays suppressed on `$z`.
        assert!(
            of_code(
                "set uri [HTTP::uri]\nset z ${uri}/suffix\nregexp $z test",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn leading_slash_concat_sets_path_prefixed() {
        // A literal leading `/` keeps the option-injection-safe PATH_PREFIXED.
        assert!(
            of_code(
                "set h [HTTP::header Host]\nset z /${h}\nregexp $z test",
                D,
                "T102",
            )
            .is_empty()
        );
    }
}

// ===========================================================================
// TestT102Suppression — PATH_PREFIXED / IP / PORT / FQDN colours suppress T102
// (cannot start with `-`); generic taint still fires it. f5-dialect sources.
// ===========================================================================
mod t102_suppression {
    use super::*;

    #[test]
    fn http_uri_no_t102() {
        // f5-dialect: HTTP::uri always starts with `/` → no option injection.
        assert!(of_code("set uri [HTTP::uri]\nregexp $uri test", D, "T102").is_empty());
    }

    #[test]
    fn http_path_no_t102() {
        assert!(of_code("set p [HTTP::path]\nregexp $p test", D, "T102").is_empty());
    }

    #[test]
    fn literal_prefix_concat_no_t102() {
        // A fixed non-dash literal prefix is option-injection-safe.
        assert!(of_code("set foo \"path_[HTTP::path]\"\nregexp $foo test", D, "T102",).is_empty());
    }

    #[test]
    fn literal_dash_prefix_concat_still_warns() {
        // A fixed `-` prefix is still option-like → warn.
        let ws = of_code("set foo \"-[HTTP::path]\"\nregexp $foo test", D, "T102");
        assert!(!ws.is_empty());
    }

    #[test]
    fn generic_taint_still_warns() {
        let ws = of_code("set x [read $fd]\nregexp $x test", D, "T102");
        assert!(!ws.is_empty());
    }

    #[test]
    fn http_header_still_warns() {
        // f5-dialect: HTTP::header is generic-tainted (no path colour) → T102.
        let ws = of_code("set h [HTTP::header Host]\nregexp $h test", D, "T102");
        assert!(!ws.is_empty());
    }

    #[test]
    fn path_prefixed_copy_suppresses() {
        assert!(of_code("set uri [HTTP::uri]\nset x $uri\nregexp $x test", D, "T102",).is_empty());
    }

    #[test]
    fn ip_client_addr_no_t102() {
        // f5-dialect: IP::client_addr starts with digit/colon → no `-` prefix.
        assert!(of_code("set addr [IP::client_addr]\nregexp $addr test", D, "T102").is_empty());
    }

    #[test]
    fn ip_remote_addr_no_t102() {
        assert!(of_code("set addr [IP::remote_addr]\nregexp $addr test", D, "T102").is_empty());
    }

    #[test]
    fn tcp_client_port_no_t102() {
        // f5-dialect: TCP::client_port is always numeric.
        assert!(of_code("set port [TCP::client_port]\nregexp $port test", D, "T102").is_empty());
    }

    #[test]
    fn tcp_remote_port_no_t102() {
        assert!(of_code("set port [TCP::remote_port]\nregexp $port test", D, "T102").is_empty());
    }

    #[test]
    fn ssl_sni_no_t102() {
        // f5-dialect: SSL::sni is an FQDN — starts with letter/digit, not `-`.
        assert!(of_code("set sni [SSL::sni]\nregexp $sni test", D, "T102").is_empty());
    }

    #[test]
    fn ip_addr_copy_preserves_colour() {
        assert!(
            of_code(
                "set addr [IP::client_addr]\nset x $addr\nregexp $x test",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn ip_addr_still_fires_t100() {
        // IP_ADDRESS does NOT suppress T100 — still dangerous for eval.
        let ws = of_code("set addr [IP::client_addr]\neval $addr", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn port_still_fires_t100() {
        let ws = of_code("set port [TCP::client_port]\neval $port", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn phi_same_colour_preserves() {
        // Both branches IP_ADDRESS → colour preserved at the phi, no T102.
        let source = "set a [IP::client_addr]\nset b [IP::remote_addr]\nif {1} {\n    set x $a\n} else {\n    set x $b\n}\nregexp $x test\n";
        assert!(of_code(source, D, "T102").is_empty());
    }

    #[test]
    fn phi_mixed_colours_preserves_non_dash() {
        // IP_ADDRESS and PORT both augment to NON_DASH_PREFIXED; T102 safe.
        let source = "set a [IP::client_addr]\nset p [TCP::client_port]\nif {$cond} {\n    set x $a\n} else {\n    set x $p\n}\nregexp $x test\n";
        assert!(of_code(source, D, "T102").is_empty());
    }

    #[test]
    fn phi_generic_with_coloured_loses() {
        // One branch generic taint, the other IP_ADDRESS → T102 fires.
        let source = "set a [IP::client_addr]\nset g [read $fd]\nif {$cond} {\n    set x $a\n} else {\n    set x $g\n}\nregexp $x test\n";
        assert!(!of_code(source, D, "T102").is_empty());
    }
}

// ===========================================================================
// Setter constraints (IRULE3101) — HTTP::uri / HTTP::path set to a value not
// provably starting with `/`. f5-dialect.
// ===========================================================================
mod setter_constraints {
    use super::*;

    #[test]
    fn literal_slash_prefix_clean() {
        assert!(of_code("HTTP::uri /newpath", IR, "IRULE3101").is_empty());
    }

    #[test]
    fn literal_no_slash_warns() {
        let ws = of_code("HTTP::uri newpath", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
        assert!(ws[0].sink_command.contains("HTTP::uri"));
    }

    #[test]
    fn path_prefixed_var_clean() {
        // f5-dialect: $uri carries PATH_PREFIXED (HTTP::uri getter).
        assert!(of_code("set uri [HTTP::uri]\nHTTP::uri $uri", IR, "IRULE3101").is_empty());
    }

    #[test]
    fn generic_tainted_var_warns() {
        let ws = of_code("set x [read $fd]\nHTTP::uri $x", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn http_path_literal_clean() {
        assert!(of_code("HTTP::path /foo/bar", IR, "IRULE3101").is_empty());
    }

    #[test]
    fn http_path_literal_warns() {
        let ws = of_code("HTTP::path relative/path", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn path_to_path_clean() {
        assert!(of_code("set p [HTTP::path]\nHTTP::path $p", IR, "IRULE3101").is_empty());
    }

    #[test]
    fn untainted_var_warns() {
        // Untainted var without a known `/` prefix → can't prove safety → warn.
        let ws = of_code("set x \"something\"\nHTTP::uri $x", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
    }
}

// ===========================================================================
// CRLF_FREE colour suppresses IRULE3002 (header) and IRULE3003
// (log). Produced by IP/PORT/FQDN sources and URI::/HTML::encode. f5-dialect.
// ===========================================================================
mod crlf_free {
    use super::*;

    #[test]
    fn ip_client_addr_augments_crlf_free() {
        assert!(
            of_code(
                "set addr [IP::client_addr]\nlog local0. $addr",
                IR,
                "IRULE3003"
            )
            .is_empty()
        );
    }

    #[test]
    fn tcp_client_port_augments_crlf_free() {
        assert!(
            of_code(
                "set port [TCP::client_port]\nlog local0. $port",
                IR,
                "IRULE3003"
            )
            .is_empty()
        );
    }

    #[test]
    fn ssl_sni_augments_crlf_free() {
        assert!(of_code("set sni [SSL::sni]\nlog local0. $sni", IR, "IRULE3003").is_empty());
    }

    #[test]
    fn uri_encode_adds_crlf_free() {
        // f5-dialect: URI::encode strips CR/LF → suppresses IRULE3003.
        assert!(
            of_code(
                "set raw [HTTP::query]\nset enc [URI::encode $raw]\nlog local0. $enc",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }

    #[test]
    fn html_encode_adds_crlf_free() {
        assert!(
            of_code(
                "set raw [HTTP::query]\nset enc [HTML::encode $raw]\nlog local0. $enc",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }

    #[test]
    fn generic_taint_irule3003_still_warns() {
        let ws = of_code("set raw [HTTP::query]\nlog local0. $raw", IR, "IRULE3003");
        assert!(!ws.is_empty());
    }

    #[test]
    fn crlf_free_suppresses_irule3002() {
        assert!(
            of_code(
                "set raw [HTTP::header Host]\nset enc [URI::encode $raw]\nHTTP::header insert X-Fwd $enc",
                IR,
                "IRULE3002",
            )
            .is_empty()
        );
    }

    #[test]
    fn crlf_free_survives_safe_concat() {
        // CRLF_FREE from an IP addr survives safe prefix/suffix
        // interpolation, so IRULE3003 stays suppressed.
        // FIXED (lattice-join identity): `word_taint` seeds the interpolated
        // value with `clean()` then joins each `$var`; with clean now the join
        // identity the first `clean().join(addr)` keeps CRLF_FREE, so IRULE3003
        // stays suppressed on `$msg`.
        assert!(
            of_code(
                "set addr [IP::client_addr]\nset msg \"src=${addr}:80\"\nlog local0. $msg",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }

    #[test]
    fn interpolation_without_crlf_preserves() {
        // Interpolation with no literal CR/LF preserves CRLF_FREE →
        // IRULE3003 suppressed.
        // FIXED (lattice-join identity): CRLF_FREE now survives the
        // interpolation `"client:${addr}"` (clean is the join identity), so
        // IRULE3003 stays suppressed.
        assert!(
            of_code(
                "set addr [IP::client_addr]\nset x \"client:${addr}\"\nlog local0. $x",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }
}

// ===========================================================================
// SHELL_ATOM augmented from IP/PORT/FQDN; does NOT suppress
// T100. f5-dialect sources.
// ===========================================================================
mod shell_atom {
    use super::*;

    #[test]
    fn ip_addr_augments_shell_atom() {
        let ws = of_code("set addr [IP::client_addr]\neval $addr", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn port_augments_shell_atom() {
        let ws = of_code("set port [TCP::client_port]\neval $port", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn fqdn_augments_shell_atom() {
        let ws = of_code("set sni [SSL::sni]\neval $sni", D, "T100");
        assert!(!ws.is_empty());
    }

    #[test]
    fn shell_atom_lost_at_phi_with_generic() {
        // Both paths tainted → T100 fires regardless of the lost colour.
        let source = "set a [IP::client_addr]\nset g [read $fd]\nif {$cond} {\n    set x $a\n} else {\n    set x $g\n}\neval $x\n";
        assert!(!of_code(source, D, "T100").is_empty());
    }
}

// ===========================================================================
// [list]/concat produce LIST_CANONICAL, but `eval $lst`
// still fires T100: the tainted first list element becomes the command word.
//
// tclsh (8.6 + 9.0): `proc marker args {puts EXECUTED}; set raw marker;
// set lst [list $raw]; eval $lst` prints `EXECUTED`. LIST_CANONICAL proves
// list-quoting, NOT that the synthesised command word is trusted (D5-T100).
// ===========================================================================
mod list_canonical {
    use super::*;

    #[test]
    fn list_command_propagates_taint_to_eval_fires() {
        // tclsh: `eval [list $raw]` runs $raw as the command word → T100.
        let ws = of_code(
            "set raw [read $fd]\nset lst [list $raw]\neval $lst",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn lsort_eval_fires() {
        // tclsh: lsort returns a canonical list but its first element is still
        // user-controlled — no command-word safety.
        let ws = of_code(
            "set raw [read $fd]\nset sorted [lsort $raw]\neval $sorted",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn lrange_eval_fires() {
        let ws = of_code(
            "set raw [read $fd]\nset sub [lrange $raw 0 2]\neval $sub",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn split_eval_fires() {
        let ws = of_code(
            "set raw [read $fd]\nset parts [split $raw :]\neval $parts",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn list_command_propagates_taint_to_puts() {
        let ws = of_code(
            "set raw [read $fd]\nset lst [list $raw]\nputs $lst",
            D,
            "T101",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn concat_of_canonical_lists_eval_fires() {
        let ws = of_code(
            "set raw [read $fd]\nset a [list $raw]\nset raw2 [read $fd2]\nset b [list $raw2]\nset c [concat $a $b]\neval $c",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn interpolation_preserves_taint_from_list() {
        let ws = of_code(
            "set raw [read $fd]\nset lst [list $raw]\nset broken \"prefix $lst\"\neval $broken",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn list_canonical_copy_eval_fires() {
        // Suppression requires a literal `[list <known-cmd> …]` at the eval site,
        // which is invisible when eval reads a propagated variable → T100 fires.
        let ws = of_code(
            "set raw [read $fd]\nset lst [list $raw]\nset copy $lst\neval $copy",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn list_canonical_copy_still_tainted() {
        let ws = of_code(
            "set raw [read $fd]\nset lst [list $raw]\nset copy $lst\nputs $copy",
            D,
            "T101",
        );
        assert!(!ws.is_empty());
    }
}

// ===========================================================================
// regex::quote / regexp::quote produce REGEX_LITERAL, lost
// on interpolation, propagated through copies. (Does not suppress T100.)
// ===========================================================================
mod regex_literal {
    use super::*;

    #[test]
    fn regex_quote_produces_colour() {
        let ws = of_code(
            "set raw [read $fd]\nset safe [regex::quote $raw]\neval $safe",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn regexp_quote_produces_colour() {
        let ws = of_code(
            "set raw [read $fd]\nset safe [regexp::quote $raw]\neval $safe",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn interpolation_invalidates_regex_literal() {
        let ws = of_code(
            "set raw [read $fd]\nset safe [regex::quote $raw]\nset pat \"prefix${safe}suffix\"\neval $pat",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn regex_literal_propagates_through_copy() {
        let ws = of_code(
            "set raw [read $fd]\nset safe [regex::quote $raw]\nset copy $safe\neval $copy",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }
}

// ===========================================================================
// `file normalize` produces PATH_NORMALISED, lost on
// interpolation, propagated through copies. (Does not suppress T100.)
// ===========================================================================
mod path_normalised {
    use super::*;

    #[test]
    fn file_normalize_produces_colour() {
        let ws = of_code(
            "set raw [read $fd]\nset norm [file normalize $raw]\neval $norm",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn interpolation_invalidates_path_normalised() {
        let ws = of_code(
            "set raw [read $fd]\nset norm [file normalize $raw]\nset broken \"${norm}/extra\"\neval $broken",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn path_normalised_propagates_through_copy() {
        let ws = of_code(
            "set raw [read $fd]\nset norm [file normalize $raw]\nset copy $norm\neval $copy",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }
}

// ===========================================================================
// HTML::encode / html_encode produce HTML_ESCAPED (+ CRLF_FREE);
// suppress IRULE3001; lost on interpolation; T106 on double-encode. f5-dialect.
// ===========================================================================
mod html_escaped {
    use super::*;

    #[test]
    fn html_encode_produces_colour() {
        assert!(of_code(
            "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nHTTP::respond 200 content $safe",
            IR,
            "IRULE3001",
        )
        .is_empty());
    }

    #[test]
    fn generic_taint_irule3001_fires() {
        let ws = of_code(
            "set raw [HTTP::query]\nHTTP::respond 200 content $raw",
            IR,
            "IRULE3001",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn interpolation_invalidates_html_escaped() {
        let ws = of_code(
            "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nset broken \"<b>${safe}</b>\"\nHTTP::respond 200 content $broken",
            IR,
            "IRULE3001",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn html_escaped_propagates_through_copy() {
        assert!(
            of_code(
                "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nset copy $safe\nHTTP::respond 200 content $copy",
                IR,
                "IRULE3001",
            )
            .is_empty()
        );
    }

    #[test]
    fn html_encode_also_sets_crlf_free() {
        assert!(
            of_code(
                "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nlog local0. $safe",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }

    #[test]
    fn html_encode_recognised_as_sanitiser() {
        // f5-dialect: html_encode (portable helper) produces HTML_ESCAPED.
        assert!(of_code(
            "set raw [HTTP::query]\nset safe [html_encode $raw]\nHTTP::respond 200 content $safe",
            IR,
            "IRULE3001",
        )
        .is_empty());
    }

    #[test]
    fn html_encode_produces_crlf_free() {
        assert!(
            of_code(
                "set raw [HTTP::query]\nset safe [html_encode $raw]\nlog local0. $safe",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }

    #[test]
    fn html_encode_double_encode_detected() {
        // T106: html_encode on already HTML_ESCAPED data.
        let ws = of_code(
            "set raw [read $fd]\nset safe [HTML::encode $raw]\nset dup [html_encode $safe]",
            IR,
            "T106",
        );
        assert!(!ws.is_empty());
    }
}

// ===========================================================================
// URI::encode / encode_component / escape produce URL_ENCODED
// (+ CRLF_FREE); lost on interpolation; propagated through copies. f5-dialect.
// ===========================================================================
mod url_encoded {
    use super::*;

    #[test]
    fn uri_encode_produces_colour() {
        let ws = of_code(
            "set raw [read $fd]\nset enc [URI::encode $raw]\neval $enc",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn uri_encode_component_produces_colour() {
        let ws = of_code(
            "set raw [read $fd]\nset enc [URI::encode_component $raw]\neval $enc",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn uri_escape_produces_colour() {
        let ws = of_code(
            "set raw [read $fd]\nset enc [URI::escape $raw]\neval $enc",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn interpolation_invalidates_url_encoded() {
        let ws = of_code(
            "set raw [read $fd]\nset enc [URI::encode $raw]\nset broken \"prefix${enc}\"\neval $broken",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn url_encoded_propagates_through_copy() {
        let ws = of_code(
            "set raw [read $fd]\nset enc [URI::encode $raw]\nset copy $enc\neval $copy",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn uri_encode_also_sets_crlf_free() {
        assert!(
            of_code(
                "set raw [HTTP::query]\nset enc [URI::encode $raw]\nlog local0. $enc",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }
}

// ===========================================================================
// CRLF_FREE in header/cookie value position suppresses
// IRULE3002; generic taint fires it. f5-dialect.
// ===========================================================================
mod header_token_safe {
    use super::*;

    #[test]
    fn crlf_free_in_header_value_suppresses_irule3002() {
        assert!(
            of_code(
                "set val [IP::client_addr]\nHTTP::header insert X-Fwd $val",
                IR,
                "IRULE3002",
            )
            .is_empty()
        );
    }

    #[test]
    fn crlf_free_in_cookie_value_suppresses_irule3002() {
        assert!(
            of_code(
                "set val [IP::client_addr]\nHTTP::cookie insert name \"track\" value $val",
                IR,
                "IRULE3002",
            )
            .is_empty()
        );
    }

    #[test]
    fn generic_taint_in_header_value_warns() {
        let ws = of_code(
            "set val [HTTP::header Host]\nHTTP::header insert X-Fwd $val",
            IR,
            "IRULE3002",
        );
        assert!(!ws.is_empty());
    }
}

// ===========================================================================
// IP/PORT/FQDN/PATH sources suppress T102;
// generic taint does not. f5-dialect sources.
// ===========================================================================
mod source_colour_augmentation {
    use super::*;

    #[test]
    fn ip_client_addr_non_dash() {
        assert!(of_code("set addr [IP::client_addr]\nregexp $addr test", D, "T102").is_empty());
    }

    #[test]
    fn tcp_client_port_non_dash() {
        assert!(of_code("set port [TCP::client_port]\nregexp $port test", D, "T102").is_empty());
    }

    #[test]
    fn ssl_sni_non_dash() {
        assert!(of_code("set sni [SSL::sni]\nregexp $sni test", D, "T102").is_empty());
    }

    #[test]
    fn path_prefixed_augments_non_dash() {
        assert!(of_code("set uri [HTTP::uri]\nregexp $uri test", D, "T102").is_empty());
    }

    #[test]
    fn generic_taint_no_augmentation() {
        let ws = of_code("set raw [read $fd]\nregexp $raw test", D, "T102");
        assert!(!ws.is_empty());
    }
}

// ===========================================================================
// Leading literal char of an interpolated
// word controls NON_DASH_PREFIXED / PATH_PREFIXED. f5-dialect sources.
// ===========================================================================
mod non_dash_prefixed_interpolation {
    use super::*;

    #[test]
    fn literal_alpha_prefix() {
        // `key_${x}` starts with `k` → NON_DASH_PREFIXED.
        assert!(
            of_code(
                "set x [HTTP::header Host]\nset y \"key_${x}\"\nregexp $y test",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn literal_digit_prefix() {
        assert!(
            of_code(
                "set x [HTTP::header Host]\nset y \"0${x}\"\nregexp $y test",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn literal_slash_prefix() {
        assert!(
            of_code(
                "set x [HTTP::header Host]\nset y \"/${x}\"\nregexp $y test",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn literal_dash_prefix_warns() {
        // `-${x}` starts with `-` → T102 fires.
        let ws = of_code(
            "set x [HTTP::header Host]\nset y \"-${x}\"\nregexp $y test",
            D,
            "T102",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn dynamic_leading_piece_inherits_original() {
        // FIXED (lattice-join identity): `${uri}/suffix` inherits PATH_PREFIXED
        // and suppresses T102 — clean is the join identity, so the interpolation
        // no longer drops the colour.
        assert!(
            of_code(
                "set uri [HTTP::uri]\nset z ${uri}/suffix\nregexp $z test",
                D,
                "T102",
            )
            .is_empty()
        );
    }

    #[test]
    fn command_subst_prefix() {
        // `path_[HTTP::path]` starts with `p` → NON_DASH_PREFIXED.
        assert!(of_code("set z \"path_[HTTP::path]\"\nregexp $z test", D, "T102").is_empty());
    }
}

// ===========================================================================
// Colour-aware interprocedural propagation.
// f5-dialect helpers.
// ===========================================================================
mod interprocedural_colours {
    use super::*;

    #[test]
    fn helper_returning_uri_encode_suppresses_irule3003() {
        // A proc that returns `[URI::encode $x]` returns a CRLF_FREE
        // value, so the caller's `log local0. $safe` is IRULE3003-suppressed.
        let source = "proc encode_it {x} { return [URI::encode $x] }\nset raw [HTTP::query]\nset safe [encode_it $raw]\nlog local0. $safe\n";
        // FIXED (lattice-join identity): the return summary joins the helper's
        // CRLF_FREE scenario onto a clean base; with clean now the join identity
        // the colour survives into the caller's `$safe`, so IRULE3003 stays
        // suppressed.
        assert!(of_code(source, IR, "IRULE3003").is_empty());
    }

    #[test]
    fn helper_returning_html_encode_suppresses_irule3001() {
        // A proc returning `[HTML::encode $x]` returns HTML_ESCAPED →
        // caller's HTTP::respond is IRULE3001-suppressed.
        let source = "proc html_safe {x} { return [HTML::encode $x] }\nset raw [HTTP::query]\nset safe [html_safe $raw]\nHTTP::respond 200 content $safe\n";
        // FIXED (lattice-join identity): the HTML_ESCAPED return scenario now
        // survives the summary join into the caller, so IRULE3001 stays
        // suppressed.
        assert!(of_code(source, IR, "IRULE3001").is_empty());
    }

    #[test]
    fn helper_passthrough_generic_taint_fires_t102() {
        let source = "proc identity {x} { return $x }\nset raw [read $fd]\nset x [identity $raw]\nregexp $x test\n";
        assert!(!of_code(source, D, "T102").is_empty());
    }

    #[test]
    fn helper_list_wrapper_eval_still_fires() {
        // tclsh: even via a helper, `eval [list $x]` runs $x as the command word.
        let source = "proc wrap_list {x} { return [list $x] }\nset raw [read $fd]\nset lst [wrap_list $raw]\neval $lst\n";
        let ws = of_code(source, D, "T100");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn helper_list_wrapper_still_tainted() {
        let source = "proc wrap_list {x} { return [list $x] }\nset raw [read $fd]\nset lst [wrap_list $raw]\nputs $lst\n";
        assert!(!of_code(source, D, "T101").is_empty());
    }

    #[test]
    fn helper_with_ip_addr_param_augmented() {
        // Passing an IP_ADDRESS (CRLF_FREE) value into a helper that
        // logs it keeps the colour at the parameter, so IRULE3003 is suppressed.
        let source =
            "proc log_addr {addr} { log local0. $addr }\nset a [IP::client_addr]\nlog_addr $a\n";
        // Passing an IP_ADDRESS (CRLF_FREE) value into a helper that logs it
        // must keep the colour at the parameter, so the in-helper
        // `log local0. $addr` is IRULE3003-suppressed.
        assert!(of_code(source, IR, "IRULE3003").is_empty());
    }
}

// ===========================================================================
// Structural colours stripped on
// interpolation; CRLF_FREE preserved (no literal CRLF). f5-dialect sources.
// ===========================================================================
mod interpolation_colour_invalidation {
    use super::*;

    #[test]
    fn list_canonical_stripped() {
        // `[list [read $fd]]` is tainted (LIST_CANONICAL), the
        // interpolation `"pre $x suf"` strips that colour, and the now-generic
        // tainted `$y` fires T100 at eval.
        let source = "set x [list [read $fd]]\nset y \"pre $x suf\"\neval $y";
        // FIXED: parse_command_substitution now splits args respecting `[...]`
        // nesting, so the `read` source nested inside `[list [read $fd]]` is
        // recovered, `$x`/`$y` are tainted, and T100 fires at `eval $y`.
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn path_normalised_stripped() {
        // `[file normalize [read $fd]]` is tainted; interpolation
        // strips PATH_NORMALISED and T100 fires at eval.
        let source = "set x [file normalize [read $fd]]\nset y \"pre $x\"\neval $y";
        // FIXED: the `read` source nested inside `[file normalize [read $fd]]`
        // is now recovered (nesting-aware arg split), so T100 fires at eval.
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn html_escaped_stripped() {
        let source = "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nset broken \"<b>$safe</b>\"\nHTTP::respond 200 content $broken";
        assert!(!of_code(source, IR, "IRULE3001").is_empty());
    }

    #[test]
    fn url_encoded_stripped() {
        let source = "set raw [read $fd]\nset enc [URI::encode $raw]\nset y \"pre$enc\"\neval $y";
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn regex_literal_stripped() {
        let source = "set raw [read $fd]\nset q [regex::quote $raw]\nset y \"^$q\"\neval $y";
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn shell_atom_stripped() {
        let source = "set addr [IP::client_addr]\nset y \"host:$addr\"\neval $y";
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn crlf_free_preserved_without_literal_crlf() {
        let source = "set addr [IP::client_addr]\nset msg \"client:${addr}\"\nlog local0. $msg";
        // FIXED (lattice-join identity): CRLF_FREE survives the interpolation, so
        // IRULE3003 stays suppressed.
        assert!(of_code(source, IR, "IRULE3003").is_empty());
    }
}

// ===========================================================================
// Transforms on untainted input stay clean;
// concat of mixed canonicality still propagates taint. f5-dialect sources.
// ===========================================================================
mod transform_colour_edge_cases {
    use super::*;

    #[test]
    fn list_with_literal_untainted() {
        // [list hello] is untainted → eval is clean.
        assert!(of_code("set x [list hello]\neval $x", D, "T100").is_empty());
    }

    #[test]
    fn concat_of_mixed_canonicality() {
        let source = "set raw [read $fd]\nset a [list $raw]\nset b [read $fd2]\nset c [concat $a $b]\neval $c";
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn uri_encode_with_untainted_no_colour() {
        // f5-dialect: URI::encode on untainted data is still untainted.
        assert!(of_code("set enc [URI::encode \"hello\"]\neval $enc", D, "T100").is_empty());
    }

    #[test]
    fn file_normalize_with_untainted_no_colour() {
        assert!(
            of_code(
                "set norm [file normalize \"/tmp/foo\"]\neval $norm",
                D,
                "T100"
            )
            .is_empty()
        );
    }
}

// ===========================================================================
// The colour→diagnostic suppression matrix.
// f5-dialect sources.
// ===========================================================================
mod sink_suppression_matrix {
    use super::*;

    #[test]
    fn irule3001_not_suppressed_by_crlf_free() {
        // URL_ENCODED/CRLF_FREE does NOT suppress IRULE3001 (XSS).
        let source =
            "set raw [HTTP::query]\nset enc [URI::encode $raw]\nHTTP::respond 200 content $enc";
        assert!(!of_code(source, IR, "IRULE3001").is_empty());
    }

    #[test]
    fn irule3001_suppressed_by_html_escaped() {
        let source =
            "set raw [HTTP::query]\nset enc [HTML::encode $raw]\nHTTP::respond 200 content $enc";
        assert!(of_code(source, IR, "IRULE3001").is_empty());
    }

    #[test]
    fn irule3002_suppressed_by_ip_address() {
        let source = "set addr [IP::client_addr]\nHTTP::header insert X-Client $addr";
        assert!(of_code(source, IR, "IRULE3002").is_empty());
    }

    #[test]
    fn irule3002_suppressed_by_port() {
        let source = "set port [TCP::client_port]\nHTTP::header insert X-Port $port";
        assert!(of_code(source, IR, "IRULE3002").is_empty());
    }

    #[test]
    fn irule3003_suppressed_by_fqdn() {
        let source = "set sni [SSL::sni]\nlog local0. $sni";
        assert!(of_code(source, IR, "IRULE3003").is_empty());
    }

    #[test]
    fn t102_not_suppressed_by_crlf_free() {
        let source = "set raw [read $fd]\nset enc [URI::encode $raw]\nregexp $enc test";
        assert!(!of_code(source, D, "T102").is_empty());
    }

    #[test]
    fn t102_not_suppressed_by_html_escaped() {
        let source = "set raw [read $fd]\nset enc [HTML::encode $raw]\nregexp $enc test";
        assert!(!of_code(source, D, "T102").is_empty());
    }

    #[test]
    fn t100_not_suppressed_by_path_prefixed() {
        let source = "set raw [HTTP::uri]\neval $raw";
        assert!(!of_code(source, D, "T100").is_empty());
    }
}

// ===========================================================================
// TestT100SinkSuppression — exec+SHELL_ATOM and eval+literal-[list-head]
// suppress T100; generic taint and propagated lists do not.
//
// tclsh facts (8.6 + 9.0):
//   * `eval [list puts $raw]` → prints `marker` (puts is the cmd word; $raw is
//     its argument) → suppressed.
//   * `eval [list $raw]`      → prints `EXECUTED` ($raw becomes the cmd word) →
//     fires.
// ===========================================================================
mod t100_sink_suppression {
    use super::*;

    #[test]
    fn exec_with_shell_atom_suppressed() {
        // f5-dialect: IP::client_addr → IP_ADDRESS augments to SHELL_ATOM, which
        // suppresses T100 for `exec` (a shell atom can't word-split).
        let source = "set addr [IP::client_addr]\nexec ping $addr";
        // FIXED: emit_sink_warnings now consults the registry's
        // taint_sink_safe_colour — exec's SHELL_ATOM (augmented from IP_ADDRESS)
        // suppresses T100 (an IP atom can't word-split an exec argument).
        assert!(of_code(source, IR, "T100").is_empty());
    }

    #[test]
    fn exec_with_generic_taint_not_suppressed() {
        let source = "set raw [read $fd]\nexec echo $raw";
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn eval_with_list_canonical_fires() {
        // tclsh: `eval [list $raw]` runs $raw as the command word.
        let source = "set raw [read $fd]\nset safe [list $raw]\neval $safe";
        let ws = of_code(source, D, "T100");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn eval_with_generic_taint_not_suppressed() {
        let source = "set raw [read $fd]\neval $raw";
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn uplevel_with_list_canonical_fires() {
        // tclsh: same hazard as eval — uplevel runs the substituted $raw.
        let source = "set raw [read $fd]\nset safe [list $raw]\nuplevel $safe";
        let ws = of_code(source, D, "T100");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "uplevel"));
    }

    #[test]
    fn eval_with_literal_list_known_head_suppressed() {
        // tclsh: `eval [list puts $raw]` → prints `marker`; $raw is the ARG, not
        // the cmd word → no code-execution vector → T100 suppressed.
        let source = "set raw [read $fd]\neval [list puts $raw]";
        assert!(of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn eval_with_literal_list_tainted_head_fires() {
        // tclsh: `eval [list $raw]` → $raw becomes the cmd word → T100 fires.
        let source = "set raw [read $fd]\neval [list $raw]";
        let ws = of_code(source, D, "T100");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn uplevel_with_generic_taint_not_suppressed() {
        let source = "set raw [read $fd]\nuplevel $raw";
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn subst_not_suppressed_by_shell_atom() {
        // subst is never suppressed by SHELL_ATOM.
        let source = "set addr [IP::client_addr]\nsubst $addr";
        assert!(!of_code(source, IR, "T100").is_empty());
    }

    #[test]
    fn expr_not_suppressed_by_list_canonical() {
        let source = "set raw [read $fd]\nset safe [list $raw]\nexpr $safe";
        assert!(!of_code(source, D, "T100").is_empty());
    }

    #[test]
    fn exec_with_port_suppressed() {
        // f5-dialect: PORT augments to SHELL_ATOM → suppresses exec T100.
        let source = "set port [TCP::client_port]\nexec firewall-cmd $port";
        // FIXED: exec's SHELL_ATOM safe colour (augmented from PORT) now
        // suppresses T100.
        assert!(of_code(source, IR, "T100").is_empty());
    }
}

// ===========================================================================
// TestT103RegexpPatternInjection — tainted data in a regexp/regsub pattern.
//
// tclsh: the pattern argument of regexp/regsub is compiled as a regex; a tainted
// pattern is a regex-injection / ReDoS vector. The string (subject) position is
// not a pattern → no T103. REGEX_LITERAL (regex::quote) suppresses.
// ===========================================================================
mod t103_regexp_pattern_injection {
    use super::*;

    #[test]
    fn tainted_regexp_pattern_fires() {
        let ws = of_code("set pat [read $fd]\nregexp $pat teststring", D, "T103");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].code.to_string(), "T103");
    }

    #[test]
    fn tainted_regsub_pattern_fires() {
        let ws = of_code(
            "set pat [read $fd]\nregsub $pat teststring replacement",
            D,
            "T103",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn tainted_regexp_with_options_fires() {
        let ws = of_code(
            "set pat [read $fd]\nregexp -nocase -- $pat teststring",
            D,
            "T103",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn regex_literal_suppresses_t103() {
        let source = "set raw [read $fd]\nset safe [regex::quote $raw]\nregexp $safe teststring";
        assert!(of_code(source, D, "T103").is_empty());
    }

    #[test]
    fn braced_regexp_argument_is_not_a_tainted_pattern() {
        assert!(
            of_code("set x [read $fd]\nregexp {$x} literal", D, "T103").is_empty(),
            "a braced dollar spelling is a literal regular expression, not a tainted use"
        );
    }

    #[test]
    fn untainted_regexp_no_warning() {
        assert!(of_code("regexp {^[a-z]+$} teststring", D, "T103").is_empty());
    }

    #[test]
    fn tainted_non_pattern_arg_no_t103() {
        // tclsh: $input is the subject (string) position, not the pattern.
        assert!(of_code("set input [read $fd]\nregexp {^\\d+$} $input", D, "T103").is_empty());
    }

    #[test]
    fn tainted_regsub_with_options_fires() {
        let ws = of_code(
            "set pat [read $fd]\nregsub -all -nocase -- $pat teststring replacement",
            D,
            "T103",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn t103_message_format() {
        let ws = of_code("set pat [read $fd]\nregexp $pat teststring", D, "T103");
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("pat"));
        assert!(ws[0].message.contains("regexp") || ws[0].message.to_lowercase().contains("regex"));
    }
}

// ===========================================================================
// PATH_NORMALISED / PATH_PREFIXED satisfy
// the IRULE3101 setter constraint. f5-dialect.
// ===========================================================================
mod path_normalised_setter_constraint {
    use super::*;

    #[test]
    fn path_normalised_suppresses_irule3101() {
        let source = "set raw [HTTP::uri]\nset norm [file normalize $raw]\nHTTP::uri $norm";
        assert!(of_code(source, IR, "IRULE3101").is_empty());
    }

    #[test]
    fn generic_taint_still_fires_irule3101() {
        let source = "set raw [HTTP::query]\nHTTP::uri $raw";
        assert!(!of_code(source, IR, "IRULE3101").is_empty());
    }

    #[test]
    fn path_prefixed_still_suppresses_irule3101() {
        let source = "set path [HTTP::path]\nHTTP::uri $path";
        assert!(of_code(source, IR, "IRULE3101").is_empty());
    }
}

// ===========================================================================
// HTTP::redirect with a tainted URL. f5-dialect.
// PATH_PREFIXED / PATH_NORMALISED (same-origin) suppress; HTML_ESCAPED does not.
// ===========================================================================
mod irule3004_open_redirect {
    use super::*;

    #[test]
    fn tainted_redirect_fires() {
        let ws = of_code(
            "set dest [HTTP::query]\nHTTP::redirect $dest",
            IR,
            "IRULE3004",
        );
        assert!(!ws.is_empty());
        assert!(ws[0].message.to_lowercase().contains("redirect"));
    }

    #[test]
    fn tainted_header_redirect_fires() {
        let ws = of_code(
            "set url [HTTP::header value Redirect-To]\nHTTP::redirect $url",
            IR,
            "IRULE3004",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn path_prefixed_suppresses_irule3004() {
        let source = "set p [HTTP::path]\nHTTP::redirect $p";
        assert!(of_code(source, IR, "IRULE3004").is_empty());
    }

    #[test]
    fn path_normalised_suppresses_irule3004() {
        let source = "set raw [HTTP::query]\nset norm [file normalize $raw]\nHTTP::redirect $norm";
        assert!(of_code(source, IR, "IRULE3004").is_empty());
    }

    #[test]
    fn html_escaped_does_not_suppress_irule3004() {
        // HTML_ESCAPED is the wrong encoding for a redirect target.
        let source = "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nHTTP::redirect $safe";
        assert!(!of_code(source, IR, "IRULE3004").is_empty());
    }

    #[test]
    fn literal_redirect_clean() {
        assert!(
            of_code(
                "HTTP::redirect \"https://example.com/home\"",
                IR,
                "IRULE3004"
            )
            .is_empty()
        );
    }

    #[test]
    fn not_in_tcl86() {
        assert!(of_code("set dest [read $fd]\nHTTP::redirect $dest", D, "IRULE3004").is_empty());
    }

    #[test]
    fn message_format() {
        let ws = of_code(
            "set url [HTTP::query]\nHTTP::redirect $url",
            IR,
            "IRULE3004",
        );
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("$url"));
        assert!(ws[0].message.contains("HTTP::redirect"));
    }
}

// ===========================================================================
// TestT106DoubleEncoding — re-encoding already-encoded data fires T106;
// cross-encode / first-encode / untainted do not. f5-dialect encoders.
// ===========================================================================
mod t106_double_encoding {
    use super::*;

    #[test]
    fn double_html_encode() {
        let source =
            "set raw [read $fd]\nset safe [HTML::encode $raw]\nset double [HTML::encode $safe]";
        let ws = of_code(source, IR, "T106");
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("HTML-escaped"));
    }

    #[test]
    fn double_uri_encode() {
        let source =
            "set raw [read $fd]\nset enc [URI::encode $raw]\nset double [URI::encode $enc]";
        let ws = of_code(source, IR, "T106");
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("URL-encoded"));
    }

    #[test]
    fn double_regex_quote() {
        let source =
            "set raw [read $fd]\nset esc [regex::quote $raw]\nset double [regex::quote $esc]";
        let ws = of_code(source, IR, "T106");
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("regex-escaped"));
    }

    #[test]
    fn cross_encode_no_fire() {
        // HTML::encode on URL_ENCODED data is a different colour → no T106.
        let source = "set raw [read $fd]\nset enc [URI::encode $raw]\nset html [HTML::encode $enc]";
        assert!(of_code(source, IR, "T106").is_empty());
    }

    #[test]
    fn first_encode_no_fire() {
        let source = "set raw [read $fd]\nset safe [HTML::encode $raw]";
        assert!(of_code(source, IR, "T106").is_empty());
    }

    #[test]
    fn double_encode_through_copy() {
        let source = "set raw [read $fd]\nset enc [URI::encode $raw]\nset copy $enc\nset double [URI::encode $copy]";
        assert!(!of_code(source, IR, "T106").is_empty());
    }

    #[test]
    fn double_encode_nested_no_intermediate_fires() {
        // FN fix: `URI::encode [URI::encode $raw]` — the inner encoder's result
        // never lands on a named variable, so the argument-word scan catches
        // it where the named-uses loop could not.
        let source = "set raw [read $fd]\nset double [URI::encode [URI::encode $raw]]";
        assert!(!of_code(source, IR, "T106").is_empty());
    }

    #[test]
    fn single_encode_nested_no_fire() {
        // A single nested encode does not double-encode.
        let source = "set raw [read $fd]\nset once [URI::encode $raw]";
        assert!(of_code(source, IR, "T106").is_empty());
    }

    #[test]
    fn untainted_no_fire() {
        // Literal data through an encoder does not fire T106.
        let source = "set x [HTML::encode \"hello\"]\nset y [HTML::encode $x]";
        assert!(of_code(source, IR, "T106").is_empty());
    }

    #[test]
    fn braced_encoder_argument_is_not_a_second_encoding() {
        let source = "set raw [read $fd]\nset x [URI::encode $raw]\nURI::encode {$x}";
        assert!(
            of_code(source, IR, "T106").is_empty(),
            "a braced dollar spelling is literal data, not a second use of the encoded variable"
        );
    }

    #[test]
    fn message_format() {
        let source = "set raw [read $fd]\nset enc [HTML::encode $raw]\nset dup [HTML::encode $enc]";
        let ws = of_code(source, IR, "T106");
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("$enc"));
        assert!(ws[0].message.contains("HTML::encode"));
        assert!(ws[0].message.contains("HTML-escaped"));
    }
}

// ===========================================================================
// TestT104NetworkSinks — tainted data in a network-address argument (SSRF).
// `socket` / `http::geturl` are core-tclsh commands.
// ===========================================================================
mod t104_network_sinks {
    use super::*;

    #[test]
    fn socket_tainted_host() {
        let ws = of_code("set host [read $fd]\nsocket $host 80", D, "T104");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "socket"));
    }

    #[test]
    fn socket_literal_clean() {
        assert!(of_code("socket localhost 80", D, "T104").is_empty());
    }

    #[test]
    fn http_geturl_tainted_url() {
        let ws = of_code("set url [read $fd]\nhttp::geturl $url", D, "T104");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "http::geturl"));
    }

    #[test]
    fn http_geturl_literal_clean() {
        assert!(of_code("http::geturl \"http://example.com\"", D, "T104").is_empty());
    }

    #[test]
    fn socket_propagation_through_copy() {
        let ws = of_code(
            "set host [read $fd]\nset h2 $host\nsocket $h2 443",
            D,
            "T104",
        );
        assert!(ws.iter().any(|w| w.variable == "h2"));
    }

    #[test]
    fn message_format() {
        let ws = of_code("set host [read $fd]\nsocket $host 80", D, "T104");
        assert!(!ws.is_empty());
        let msg = &ws[0].message;
        assert!(msg.contains("SSRF"));
        assert!(msg.contains("socket"));
    }
}

// ===========================================================================
// TestT105InterpEvalSinks — tainted data in an interp eval/invokehidden script.
// `interp eval` is a core-tclsh command. LIST_CANONICAL does NOT suppress; a
// literal [list <known-cmd> …] head does (same tclsh facts as the eval cases).
// ===========================================================================
mod t105_interp_eval_sinks {
    use super::*;

    #[test]
    fn interp_eval_tainted() {
        let ws = of_code(
            "set script [read $fd]\ninterp eval $child $script",
            D,
            "T105",
        );
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command.contains("interp eval")));
    }

    #[test]
    fn interp_eval_literal_clean() {
        assert!(of_code("interp eval $child \"puts hello\"", D, "T105").is_empty());
    }

    #[test]
    fn interp_invokehidden_tainted() {
        let ws = of_code(
            "set cmd [read $fd]\ninterp invokehidden $child $cmd",
            D,
            "T105",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn list_does_not_suppress_t105() {
        // tclsh: same hazard as eval — the child re-parses $safe and the first
        // list element becomes the command word.
        let source = "set x [read $fd]\nset safe [list $x]\ninterp eval $child $safe";
        let ws = of_code(source, D, "T105");
        assert!(!ws.is_empty());
        assert!(ws.iter().any(|w| w.sink_command == "interp eval"));
    }

    #[test]
    fn interp_eval_literal_list_known_head_suppressed() {
        // tclsh: literal `[list puts $x]` → puts is the cmd word, $x the arg.
        let source = "set x [read $fd]\ninterp eval $child [list puts $x]";
        assert!(of_code(source, D, "T105").is_empty());
    }

    #[test]
    fn message_format() {
        let ws = of_code("set data [read $fd]\ninterp eval $child $data", D, "T105");
        assert!(!ws.is_empty());
        let msg = &ws[0].message;
        assert!(msg.contains("interp eval"));
        assert!(msg.to_lowercase().contains("injection"));
    }
}

// ===========================================================================
// Splitting HTTP::uri on `?` / `&` instead of using
// HTTP::path / HTTP::query. f5-dialect.
// ===========================================================================
mod irule3103_uri_split {
    use super::*;

    #[test]
    fn split_uri_on_question_mark() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset parts [split $uri \"?\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn split_uri_on_ampersand() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset parts [split $uri \"&\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn split_uri_on_question_and_ampersand() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset parts [split $uri \"?&\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn inline_command_substitution() {
        let ws = of_code("set parts [split [HTTP::uri] \"?\"]", IR, "IRULE3103");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn split_non_uri_clean() {
        let ws = of_code(
            "set x \"foo?bar\"\nset parts [split $x \"?\"]",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn split_http_path_clean() {
        let ws = of_code(
            "set p [HTTP::path]\nset parts [split $p \"?\"]",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn split_uri_on_slash_clean() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset parts [split $uri \"/\"]",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn copy_propagation() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset copy $uri\nset parts [split $copy \"?\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn tcl_dialect_clean() {
        let ws = of_code("set x \"foo\"\nset parts [split $x \"?\"]", D, "IRULE3103");
        assert!(ws.is_empty());
    }

    #[test]
    fn uri_setter_not_flagged() {
        // The HTTP::uri setter form (with arg) is not a getter origin.
        let ws = of_code(
            "HTTP::uri \"/new\"\nset parts [split [HTTP::uri /path] \"?\"]",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }
}

// ===========================================================================
// Expression operators on HTTP::uri suggesting
// HTTP::path / HTTP::query. f5-dialect.
// ===========================================================================
mod irule3103_expr_operators {
    use super::*;

    #[test]
    fn starts_with_path_pattern() {
        let ws = of_code(
            "if { [HTTP::uri] starts_with \"/api\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("starts_with"));
    }

    #[test]
    fn starts_with_via_variable() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { $uri starts_with \"/api\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn ends_with_extension() {
        let ws = of_code(
            "if { [HTTP::uri] ends_with \".html\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn contains_query_param() {
        let ws = of_code(
            "if { [HTTP::uri] contains \"&key=\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn contains_equals_query() {
        let ws = of_code(
            "if { [HTTP::uri] contains \"user=test\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn matches_glob_path() {
        let ws = of_code(
            "if { [HTTP::uri] matches_glob \"/api/*\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn matches_glob_query() {
        let ws = of_code(
            "if { [HTTP::uri] matches_glob \"*&key=*\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn non_uri_clean() {
        let ws = of_code(
            "set p [HTTP::path]\nif { $p starts_with \"/api\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn ambiguous_operand_clean() {
        let ws = of_code(
            "if { [HTTP::uri] contains \"something\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }
}

// ===========================================================================
// string match / string first on HTTP::uri.
// f5-dialect.
// ===========================================================================
mod irule3103_string_match {
    use super::*;

    #[test]
    fn string_match_path_pattern() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"/api/*\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn string_match_query_pattern() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"*&key=*\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn string_first_question_mark() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset pos [string first \"?\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path") || ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn string_match_non_uri_clean() {
        let ws = of_code(
            "set p [HTTP::path]\nset m [string match \"/api/*\" $p]",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn string_match_ambiguous_pattern_clean() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"*something*\" $uri]",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn string_match_in_if_condition() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { [string match \"/static/*\" $uri] } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn string_first_in_if_condition() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { [string first \"?\" $uri] >= 0 } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
    }
}

// ===========================================================================
// glob/regex classifier edge cases and SSA correctness.
// f5-dialect.
//
// tclsh-semantic premises (glob/regex metacharacter meanings): glob `?` is a
// single-char wildcard (not a query delimiter); regex `?` is a quantifier; an
// escaped `\?` / `\&` is a literal char (query signal). These are about how a
// pattern's characters classify, not a runtime side-effect to observe.
// ===========================================================================
mod irule3103_edge_cases {
    use super::*;

    #[test]
    fn glob_question_mark_is_wildcard_not_query() {
        // glob `??` is a wildcard; `/api/??` is path-like (starts with `/`).
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"/api/??\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn glob_bare_question_not_query() {
        // Bare `??` wildcard with no path prefix → ambiguous → no fire.
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"??\" $uri]",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn glob_question_path_with_wildcard() {
        let ws = of_code(
            "if { [HTTP::uri] matches_glob \"/api/?\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn regex_question_is_quantifier_not_query() {
        // regex `?` is a quantifier; `/api/v[0-9]+/?` is path-like.
        let ws = of_code(
            "if { [HTTP::uri] matches_regex \"^/api/v[0-9]+/?$\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn regex_escaped_question_is_query() {
        // regex `\?` is a literal `?` → query signal.
        let ws = of_code(
            "if { [HTTP::uri] matches_regex \"\\\\?key=\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn later_reassignment_no_false_positive() {
        // $uri is "foo" at the expr check; reassigned to HTTP::uri afterwards.
        let ws = of_code(
            "set uri \"foo\"\nset ok [expr { $uri starts_with \"/api\" }]\nset uri [HTTP::uri]",
            IR,
            "IRULE3103",
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn string_match_nocase_in_if_condition() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { [string match -nocase \"/STATIC/*\" $uri] } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn split_separator_from_constant_variable() {
        // SCCP-resolved separator variable still triggers the split warning.
        let ws = of_code(
            "set uri [HTTP::uri]\nset sep \"?\"\nset parts [split $uri $sep]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn phi_mixed_uri_and_non_uri_no_warning() {
        // Mixed-origin phi (URI + non-URI) → no warning.
        let source = "set cond 0\nif { $cond } {\n    set candidate [HTTP::uri]\n} else {\n    set candidate \"/not-uri\"\n}\nset out [string match \"/api/*\" $candidate]";
        assert!(of_code(source, IR, "IRULE3103").is_empty());
    }

    #[test]
    fn string_first_with_ampersand_in_if_condition() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { [string first \"&\" $uri] >= 0 } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn split_separator_from_copied_constant_variable() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset sep \"?\"\nset sep_copy $sep\nset parts [split $uri $sep_copy]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn nested_boolean_expression_emits_one_hit_per_uri_use() {
        // One warning per concrete URI-pattern use.
        let source = "set uri [HTTP::uri]\nif { ([string match \"/api/*\" $uri] && ([HTTP::uri] contains \"&id=\")) } {\n    log local0. ok\n}";
        let ws = of_code(source, IR, "IRULE3103");
        assert_eq!(ws.len(), 2);
        let mut sinks: Vec<String> = ws.iter().map(|w| w.sink_command.clone()).collect();
        sinks.sort();
        assert_eq!(
            sinks,
            vec!["contains".to_string(), "string match".to_string()]
        );
    }

    #[test]
    fn phi_both_branches_uri_still_warns() {
        let source = "set cond 1\nif { $cond } {\n    set candidate [HTTP::uri]\n} else {\n    set tmp [HTTP::uri]\n    set candidate $tmp\n}\nset out [string match \"/api/*\" $candidate]";
        let ws = of_code(source, IR, "IRULE3103");
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn regex_escaped_ampersand_is_query() {
        // regex `\&` is a literal `&` → query signal.
        let ws = of_code(
            "if { [HTTP::uri] matches_regex \"key\\\\&id\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }
}

// ===========================================================================
// Interpreter special-variable taint sources (`env`, `argv`, `argv0`).
//
// Reading the process environment or the command line is attacker-influenced
// external input, seeded from the dialect-aware special-variable registry
// (`tcl_registry::special_vars`). The restricted iRules interpreter provides
// none of them, so they are sources only in the standard Tcl dialects.
// ===========================================================================
mod special_variable_sources {
    use super::*;

    #[test]
    fn env_read_flows_tainted_into_code_sink() {
        let ws = of_code("eval $env(CMD)", D, "T100");
        assert!(!ws.is_empty(), "eval of $env(...) should fire T100");
    }

    #[test]
    fn argv_read_flows_tainted_into_code_sink() {
        let ws = of_code("eval $argv", D, "T100");
        assert!(!ws.is_empty(), "eval of $argv should fire T100");
    }

    #[test]
    fn argv0_read_flows_tainted_into_code_sink() {
        let ws = of_code("eval $argv0", D, "T100");
        assert!(!ws.is_empty(), "eval of $argv0 should fire T100");
    }

    #[test]
    fn local_shadow_is_not_a_source() {
        // A local `set argv …` shadows the interpreter global: the read sees
        // the clean literal (higher SSA version), so no taint reaches the sink.
        assert_eq!(codes("set argv clean\neval $argv", D), Vec::<String>::new());
    }

    #[test]
    fn env_is_not_a_source_in_irules() {
        // The restricted iRules interpreter has no `env`, so it is not seeded
        // as a taint source there.
        assert!(of_code("eval $env(CMD)", IR, "T100").is_empty());
    }
}
