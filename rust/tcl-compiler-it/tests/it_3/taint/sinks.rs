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

//! Dangerous sinks and their messages: evaluation, output, logs, network and interpreter sinks, and the open redirect.

// Dangerous sinks (T100) — tainted data in eval/expr/exec/uplevel/subst.
//
// tclsh: each of these re-parses/executes its argument, so a tainted value is a
// code-execution vector. (eval/uplevel/subst/exec run the value; unbraced expr
// re-parses it.)
mod dangerous_sinks {
    use super::super::*;

    #[test]
    fn eval_sink() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\neval $x", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].sink_command, "eval");
    }

    #[test]
    fn expr_sink() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nexpr $x", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn exec_sink() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nexec $x", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].sink_command, "exec");
    }

    #[test]
    fn uplevel_sink() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nuplevel $x", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn subst_sink() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nsubst $x", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn coroprobe_command_prefix_sink() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // `coroprobe coroName command ?arg ...?` evaluates `command` in a
        // suspended coroutine's frame right now — it carries
        // `Traits::EVALUATES_CODE` but (unlike `eval`/`exec`/`uplevel`) not
        // `Traits::TAINT_SINK`, so the registry-driven sink classification
        // must still route it to T100 via the `EVALUATES_CODE` trait rather
        // than silently losing it now that sink classification is
        // registry-driven instead of hardcoded per-trait checks in the
        // compiler.
        let ws = channel_of_code(
            "set cmd [read $fd]\ncoroprobe myCoro $cmd",
            "tcl9.0",
            "T100",
        );
        assert!(!ws.is_empty(), "expected T100 for coroprobe, got none");
    }

    #[test]
    fn coroinject_command_prefix_sink() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set cmd [read $fd]\ncoroinject myCoro $cmd",
            "tcl9.0",
            "T100",
        );
        assert!(!ws.is_empty(), "expected T100 for coroinject, got none");
    }
}

// The T100 message names the sink and the variable.
mod warning_messages {
    use super::super::*;

    #[test]
    fn taint_warning_message() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\neval $x", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].message.contains("eval"));
        assert!(ws[0].message.contains("$x") || ws[0].message.contains('x'));
    }
}

// Output sinks (T101) — tainted data flowing into `puts`.
//
// tclsh: `puts ?-nonewline? ?channelId? string` — only the trailing content
// word is output content; a tainted channel id is a destination handle and must
// not flag T101.
mod output_sinks {
    use super::super::*;

    #[test]
    fn puts_tainted_data_span_is_tight_around_argument() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // The diagnostic must underline just the tainted `$x` argument word,
        // not the whole `puts $x` statement — precise highlighting is what
        // lets the developer see exactly which value is the problem.
        let src = "set fd stdin\nset x [read $fd]\nputs $x";
        let ws = dialect_of_code(src, D, "T101");
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
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nputs $x", D, "T101");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].variable, "x");
        assert!(ws[0].sink_command.contains("puts"));
    }

    #[test]
    fn puts_literal_clean() {
        crate::install_surfaces();
        assert_eq!(
            of_code("puts \"hello world\"", D, "T101"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn puts_sanitised_clean() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        assert_eq!(
            channel_of_code(
                "set x [read $fd]\nset n [string length $x]\nputs $n",
                D,
                "T101",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn puts_tainted_channel_position_silent() {
        crate::install_surfaces();
        // tclsh: in `puts -nonewline $chan {hello}`, $chan is the destination
        // channel, not injectable content (tcllib imap4.tcl idiom). No T101.
        assert_eq!(
            of_code(
                "set chan [read $fd]\nputs -nonewline $chan {hello}",
                D,
                "T101",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn t100_tainted_inside_cmd_subst_in_expr_silent() {
        crate::install_surfaces();
        // tclsh: in `expr {([string length $data] / 8) * 8}`, $data is consumed
        // by the inner `string length`; expr sees only the int result, not the
        // original $data. No injection risk → T100 must not fire on $data.
        assert_eq!(
            of_code(
                "set data [read $fd]\nset n [expr {([string length $data] / 8) * 8}]",
                D,
                "T100",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn t100_tainted_direct_expr_operand_still_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // Control: $data as a DIRECT expr operand IS the injection vector.
        let ws = channel_of_code("set data [read $fd]\nset v [expr {$data + 1}]", D, "T100");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "data");
    }

    #[test]
    fn t100_tainted_expr_func_arg_still_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // `abs($data)` — a math-func arg is a direct expr operand → T100 fires.
        let ws = channel_of_code("set data [read $fd]\nset v [expr {abs($data)}]", D, "T100");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn puts_tainted_output_alongside_tainted_chan() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // Control: when BOTH channel and content are tainted, only the content
        // word flags T101.
        let ws = channel_of_code(
            "set chan [read $fd]\nset msg [read $fd]\nputs -nonewline $chan $msg",
            D,
            "T101",
        );
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "msg");
    }

    #[test]
    fn puts_interpolation_propagates() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nputs \"data: $x\"", D, "T101");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn puts_nonewline_alone_is_content_not_channel() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // `puts -nonewline $x` has only ONE positional arg after the flag,
        // so per Tcl semantics `$x` is the content written to stdout, not a
        // channel — the `-nonewline`-shifted `ArgRole::Channel` position
        // (declared dynamically since the channel slot moves with the
        // optional flag) must not exempt it.
        let ws = channel_of_code("set x [read $fd]\nputs -nonewline $x", D, "T101");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "x");
    }

    #[test]
    fn puts_via_interp_alias_still_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // `interp alias {} myputs {} puts` makes `myputs` a genuine
        // current-interpreter alias for the real `puts` builtin. The sink
        // classification must resolve the call through the lowerer's
        // `canonical_command` (populated for a resolved alias) rather than
        // the literal source spelling `"myputs"`, which isn't itself a
        // registered command and would otherwise silently miss the sink.
        let ws = channel_of_code(
            "interp alias {} myputs {} puts\nset x [read $fd]\nmyputs $x",
            D,
            "T101",
        );
        assert_eq!(ws.len(), 1, "expected T101 via interp alias, got {ws:?}");
        assert_eq!(ws[0].variable, "x");
    }

    #[test]
    fn puts_inside_tcloo_method_body_still_fires() {
        crate::install_surfaces();
        // TclOO method bodies are analysable functions like any proc — the
        // sink check must not be limited to top-level / plain-proc bodies.
        // Qualify the core heads because an instance method executes in its
        // runtime-selected object namespace: a relative `puts` may denote an
        // object-local command, for which deep analysis correctly abstains.
        // The channel is the method's formal: tclsh errors on an undefined
        // method-local `$fd`.
        let ws = of_code(
            "oo::class create Foo {\n\
             method bar {fd} {\n\
             ::set x [::read $fd]\n\
             ::puts $x\n\
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
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // `namespace eval` registers its block as a synthetic body unit
        // (`Module::body_units`) — a fresh-frame body outside any proc —
        // which must be covered by the same sink check as the top level.
        let ws = channel_of_code(
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

// IRULE3001 (HTTP::respond body) / IRULE3002
// (HTTP::header|cookie insert|replace). f5-dialect; gated on the iRules dialect.
mod irules_output_sinks {
    use super::super::*;

    #[test]
    fn http_respond_tainted_body() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // f5-dialect.
        let ws = dialect_of_code(
            "set data [HTTP::payload]\nHTTP::respond 200 content $data",
            IR,
            "IRULE3001",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].sink_command.contains("HTTP::respond"));
    }

    #[test]
    fn http_respond_literal_clean() {
        crate::install_surfaces();
        assert_eq!(
            of_code("HTTP::respond 200 content \"ok\"", IR, "IRULE3001"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn http_header_insert_tainted() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // f5-dialect.
        let ws = dialect_of_code(
            "set val [HTTP::header Host]\nHTTP::header insert X-Foo $val",
            IR,
            "IRULE3002",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].sink_command.to_lowercase().contains("header"));
    }

    #[test]
    fn http_header_remove_clean() {
        crate::install_surfaces();
        // `remove` is not an insert/replace value sink.
        assert_eq!(
            of_code("HTTP::header remove X-Bad", IR, "IRULE3002"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn http_cookie_insert_tainted() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // f5-dialect.
        let ws = dialect_of_code(
            "set val [HTTP::cookie value session]\nHTTP::cookie insert name \"track\" value $val",
            IR,
            "IRULE3002",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn not_in_tcl86_dialect() {
        crate::install_surfaces();
        // Under plain tcl8.6 the HTTP::respond sink is not classified → no
        // IRULE3001 even though `read` taints `$data`.
        assert_eq!(
            of_code(
                "set data [read $fd]\nHTTP::respond 200 content $data",
                D,
                "IRULE3001",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// Log injection (IRULE3003) — tainted data in `log`. f5-dialect.
mod log_injection {
    use super::super::*;

    #[test]
    fn log_tainted_data() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // f5-dialect.
        let ws = dialect_of_code("set x [HTTP::uri]\nlog local0. $x", IR, "IRULE3003");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].sink_command.contains("log"));
    }

    #[test]
    fn log_literal_clean() {
        crate::install_surfaces();
        assert_eq!(
            of_code("log local0. \"static message\"", IR, "IRULE3003"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn log_not_in_tcl86() {
        crate::install_surfaces();
        assert_eq!(
            of_code("set x [read $fd]\nlog local0. $x", D, "IRULE3003"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// HTTP::redirect with a tainted URL. f5-dialect.
// PATH_PREFIXED / PATH_NORMALISED (same-origin) suppress; HTML_ESCAPED does not.
mod irule3004_open_redirect {
    use super::super::*;

    #[test]
    fn tainted_redirect_fires() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "set dest [HTTP::query]\nHTTP::redirect $dest",
            IR,
            "IRULE3004",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].message.to_lowercase().contains("redirect"));
    }

    #[test]
    fn tainted_header_redirect_fires() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "set url [HTTP::header value Redirect-To]\nHTTP::redirect $url",
            IR,
            "IRULE3004",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn path_prefixed_suppresses_irule3004() {
        crate::install_surfaces();
        let source = "set p [HTTP::path]\nHTTP::redirect $p";
        assert_eq!(
            of_code(source, IR, "IRULE3004"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn normalised_path_target_suppresses_irule3004() {
        crate::install_surfaces();
        // TMM has no `file`; its normaliser is the getter's `-normalized` flag.
        let source =
            "when HTTP_REQUEST {\nset norm [HTTP::path -normalized]\nHTTP::redirect $norm\n}";
        assert_eq!(
            of_code(source, IR, "IRULE3004"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn html_escaped_does_not_suppress_irule3004() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // HTML_ESCAPED is the wrong encoding for a redirect target.
        let source = "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nHTTP::redirect $safe";
        assert_ne!(
            dialect_of_code(source, IR, "IRULE3004"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn literal_redirect_clean() {
        crate::install_surfaces();
        assert_eq!(
            of_code(
                "HTTP::redirect \"https://example.com/home\"",
                IR,
                "IRULE3004"
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn not_in_tcl86() {
        crate::install_surfaces();
        assert_eq!(
            of_code("set dest [read $fd]\nHTTP::redirect $dest", D, "IRULE3004"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn message_format() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "set url [HTTP::query]\nHTTP::redirect $url",
            IR,
            "IRULE3004",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].message.contains("$url"));
        assert!(ws[0].message.contains("HTTP::redirect"));
    }
}

// TestT104NetworkSinks — tainted data in a network-address argument (SSRF).
// `socket` / `http::geturl` are core-tclsh commands.
mod t104_network_sinks {
    use super::super::*;

    #[test]
    fn socket_tainted_host() {
        crate::install_surfaces();
        let ws = of_code("set host [read $fd]\nsocket $host 80", D, "T104");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "socket"));
    }

    #[test]
    fn socket_literal_clean() {
        crate::install_surfaces();
        assert_eq!(
            of_code("socket localhost 80", D, "T104"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn http_geturl_tainted_url() {
        crate::install_surfaces();
        // Dialect build with a defined channel and the package loaded: tclsh
        // errors on an undefined `$fd` and on `http::geturl` before
        // `package require http`.
        let ws = channel_of_code(
            "package require http\nset url [read $fd]\nhttp::geturl $url",
            D,
            "T104",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "http::geturl"));
    }

    #[test]
    fn http_geturl_literal_clean() {
        crate::install_surfaces();
        assert_eq!(
            of_code("http::geturl \"http://example.com\"", D, "T104"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn socket_propagation_through_copy() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set host [read $fd]\nset h2 $host\nsocket $h2 443",
            D,
            "T104",
        );
        assert!(ws.iter().any(|w| w.variable == "h2"));
    }

    #[test]
    fn message_format() {
        crate::install_surfaces();
        let ws = of_code("set host [read $fd]\nsocket $host 80", D, "T104");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        let msg = &ws[0].message;
        assert!(msg.contains("SSRF"));
        assert!(msg.contains("socket"));
    }
}

// TestT105InterpEvalSinks — tainted data in an interp eval/invokehidden script.
// `interp eval` is a core-tclsh command. LIST_CANONICAL does NOT suppress; a
// literal [list <known-cmd> …] head does (same tclsh facts as the eval cases).
mod t105_interp_eval_sinks {
    use super::super::*;

    #[test]
    fn interp_eval_tainted() {
        crate::install_surfaces();
        let ws = of_code(
            "set script [read $fd]\ninterp eval $child $script",
            D,
            "T105",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command.contains("interp eval")));
    }

    #[test]
    fn interp_eval_literal_clean() {
        crate::install_surfaces();
        // Dialect build with a defined `child`: tclsh errors on the undefined one.
        assert_eq!(
            dialect_of_code(
                "set child [interp create]\ninterp eval $child \"puts hello\"",
                D,
                "T105"
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn interp_invokehidden_tainted() {
        crate::install_surfaces();
        let ws = of_code(
            "set cmd [read $fd]\ninterp invokehidden $child $cmd",
            D,
            "T105",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn list_does_not_suppress_t105() {
        crate::install_surfaces();
        // tclsh: same hazard as eval — the child re-parses $safe and the first
        // list element becomes the command word.
        // Tcl requires a defined child and channel before the nested evaluation.
        let source = "set child [interp create]\nset x [read $fd]\nset safe [list $x]\ninterp eval $child $safe";
        let ws = channel_of_code(source, D, "T105");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "interp eval"));
    }

    #[test]
    fn interp_eval_literal_list_known_head_suppressed() {
        crate::install_surfaces();
        // Tcl requires a defined child and channel before the nested evaluation.
        // A literal head makes the tainted value an argument, not the command.
        let prefix = "set child [interp create]\nset x [read $fd]\n";
        let source = format!("{prefix}interp eval $child [list puts $x]");
        assert_eq!(
            channel_of_code(&source, D, "T105"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
        // A tainted command head in the same reached child evaluation must warn.
        let control = format!("{prefix}interp eval $child [list $x marker]");
        let warnings = channel_of_code(&control, D, "T105");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.sink_command.contains("interp eval"))
        );
    }

    #[test]
    fn message_format() {
        crate::install_surfaces();
        let ws = of_code("set data [read $fd]\ninterp eval $child $data", D, "T105");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        let msg = &ws[0].message;
        assert!(msg.contains("interp eval"));
        assert!(msg.to_lowercase().contains("injection"));
    }
}
