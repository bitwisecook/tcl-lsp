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
use tcl_registry::registry_for_dialect;

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
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
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
        assert!(
            of_code(
                "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nHTTP::respond 200 content $safe",
                IR,
                "IRULE3001",
            )
            .is_empty()
        );
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
        assert!(
            of_code(
                "set raw [HTTP::query]\nset safe [html_encode $raw]\nHTTP::respond 200 content $safe",
                IR,
                "IRULE3001",
            )
            .is_empty()
        );
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
