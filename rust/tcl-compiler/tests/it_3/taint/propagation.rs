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

//! Propagation through assignments and control flow, and across procedure calls with their colours.

// Taint flows through assignments and interpolation.
mod taint_propagation {
    use super::super::*;

    #[test]
    fn variable_copy_propagates() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nset y $x\neval $y", D, "T100");
        assert!(ws.iter().any(|w| w.variable == "y"));
    }

    #[test]
    fn string_interpolation_propagates() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set x [read $fd]\nset y \"prefix${x}suffix\"\neval $y",
            D,
            "T100",
        );
        assert!(ws.iter().any(|w| w.variable == "y"));
    }

    #[test]
    fn command_subst_concat_propagates() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]/suffix\neval $x", D, "T100");
        assert!(ws.iter().any(|w| w.variable == "x"));
    }

    #[test]
    fn sanitiser_blocks_taint() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: `string length $x` returns an int (`11` for an 11-char value) —
        // a fixed numeric return cannot carry taint, so the result is clean.
        assert_eq!(
            channel_codes("set x [read $fd]\nset n [string length $x]\nexpr $n", D),
            Vec::<String>::new()
        );
    }

    #[test]
    fn llength_sanitises() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: `llength` returns an int (element count) — sanitises taint.
        assert_eq!(
            channel_codes("set x [read $fd]\nset n [llength $x]\nexpr $n", D),
            Vec::<String>::new()
        );
    }

    #[test]
    fn taint_through_if_branch() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source =
            "set x [read $fd]\nif {1} {\n    set y $x\n} else {\n    set y \"safe\"\n}\neval $y\n";
        let ws = channel_of_code(source, D, "T100");
        assert!(ws.iter().any(|w| w.variable == "y"));
    }
}

// Proc summaries propagate taint across calls.
mod interprocedural_taint {
    use super::super::*;

    #[test]
    fn helper_sanitiser_suppresses_taint() {
        // tclsh: the helper returns `string length` (an int) → clean result.
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source = "set fd stdin\nproc safe_len {x} { return [string length $x] }\nset raw [read $fd]\nset n [safe_len $raw]\nexpr $n\n";
        let mut codes: Vec<String> = dialect_warns(source, D)
            .iter()
            .map(|w| w.code.to_string())
            .collect();
        codes.sort();
        assert_eq!(codes, Vec::<String>::new());
        let control = format!("{source}puts $raw\n");
        let control_warnings = dialect_warns(&control, D);
        assert!(
            control_warnings.iter().any(|warning| {
                warning.code.to_string() == "T101"
                    && warning.variable == "raw"
                    && warning.sink_command == "puts"
            }),
            "same-prefix positive control: {control_warnings:?}"
        );
    }

    #[test]
    fn tainted_return_from_helper_reaches_sink() {
        // f5-dialect: helper returns HTTP::payload (tainted) → flows to eval.
        let source = "proc payload {} { return [HTTP::payload] }\nset data [payload]\neval $data\n";
        let ws = of_code(source, IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.variable == "data"));
    }

    #[test]
    fn tainted_argument_into_sinking_helper_warns() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source =
            "set fd stdin\nproc do_eval {x} { eval $x }\nset data [read $fd]\ndo_eval $data\n";
        let ws: Vec<_> = dialect_warns(source, D)
            .into_iter()
            .filter(|w| w.code.to_string() == "T100")
            .collect();
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn tainted_variadic_args_into_sinking_helper_warns() {
        // `proc log {args} { puts $args }` packs every trailing actual
        // argument into the `args` list parameter — a tainted actual must
        // still be traced (via the interprocedural entry-taint solve's
        // `args`-packing rule) into the sink inside the callee body.
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source =
            "set fd stdin\nproc log {args} { puts $args }\nset data [read $fd]\nlog $data\n";
        let ws: Vec<_> = dialect_warns(source, D)
            .into_iter()
            .filter(|w| w.code.to_string() == "T101")
            .collect();
        assert!(
            !ws.is_empty(),
            "expected T101 for tainted args-packed data, got {ws:?}"
        );
    }
}

// Colour-aware interprocedural propagation.
// f5-dialect helpers.
mod interprocedural_colours {
    use super::super::*;

    #[test]
    fn helper_returning_uri_encode_suppresses_irule3003() {
        // A proc that returns `[URI::encode $x]` returns a CRLF_FREE
        // value, so the caller's `log local0. $safe` is IRULE3003-suppressed.
        let source = "proc encode_it {x} { return [URI::encode $x] }\nset raw [HTTP::query]\nset safe [encode_it $raw]\nlog local0. $safe\n";
        // FIXED (lattice-join identity): the return summary joins the helper's
        // CRLF_FREE scenario onto a clean base; with clean now the join identity
        // the colour survives into the caller's `$safe`, so IRULE3003 stays
        // suppressed.
        assert_eq!(
            of_code(source, IR, "IRULE3003"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn helper_returning_html_encode_suppresses_irule3001() {
        // A proc returning `[HTML::encode $x]` returns HTML_ESCAPED →
        // caller's HTTP::respond is IRULE3001-suppressed.
        let source = "proc html_safe {x} { return [HTML::encode $x] }\nset raw [HTTP::query]\nset safe [html_safe $raw]\nHTTP::respond 200 content $safe\n";
        // FIXED (lattice-join identity): the HTML_ESCAPED return scenario now
        // survives the summary join into the caller, so IRULE3001 stays
        // suppressed.
        assert_eq!(
            of_code(source, IR, "IRULE3001"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn helper_passthrough_generic_taint_fires_t102() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source = "proc identity {x} { return $x }\nset raw [read $fd]\nset x [identity $raw]\nregexp $x test\n";
        assert_ne!(
            channel_of_code(source, D, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn helper_list_wrapper_eval_still_fires() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: even via a helper, `eval [list $x]` runs $x as the command word.
        let source = "proc wrap_list {x} { return [list $x] }\nset raw [read $fd]\nset lst [wrap_list $raw]\neval $lst\n";
        let ws = channel_of_code(source, D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn helper_list_wrapper_still_tainted() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source = "proc wrap_list {x} { return [list $x] }\nset raw [read $fd]\nset lst [wrap_list $raw]\nputs $lst\n";
        assert_ne!(
            channel_of_code(source, D, "T101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
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
        assert_eq!(
            of_code(source, IR, "IRULE3003"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}
