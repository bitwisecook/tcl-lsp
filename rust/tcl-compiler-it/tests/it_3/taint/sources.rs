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

//! Taint sources: Tcl channels and commands, iRules getters, special variables and the colours a source adds.

// Tcl-core I/O commands taint their results (T100 at the
// eval sink). `read`/`gets`/`exec`/`socket`/`chan` are real tclsh commands.
mod tcl_taint_sources {
    use super::super::*;

    #[test]
    fn read_taints() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: `read $fd` returns channel contents — attacker-influenced I/O.
        let ws = channel_of_code("set data [read $fd]\neval $data", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].variable, "data");
    }

    #[test]
    fn gets_taints() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: `gets $fd` (no varName) returns the line read from the channel.
        let ws = channel_of_code("set line [gets $fd]\neval $line", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].variable, "line");
    }

    #[test]
    fn a_call_the_module_cannot_see_leaves_what_its_words_read_tainted() {
        crate::install_surfaces();
        // `foo` may rewrite `line`, but nothing it does cleans the value the
        // read before it held.
        let ws = of_code("set line [gets stdin]\nfoo $line\neval $line", D, "T100");
        assert!(
            ws.iter().any(|warning| warning.variable == "line"),
            "the read after the call is as tainted as the read before it: {ws:?}"
        );
    }

    #[test]
    fn constant_variable_command_head_reaches_sink_at_the_use_site() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
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
        crate::install_surfaces();
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
        crate::install_surfaces();
        assert!(
            of_code("set x [gets stdin]\n{[eval $x]}", D, "T100").is_empty(),
            "a braced command name is literal; its bracket text is never evaluated"
        );
    }

    #[test]
    fn quoted_whole_command_substitution_reaches_inner_sink() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code("set x [gets stdin]\n\"[eval $x]\"", D, "T100");
        assert!(
            ws.iter().any(|warning| warning.variable == "x"),
            "quotes do not suppress a whole command substitution in command position: {ws:?}"
        );
    }

    #[test]
    fn escaped_static_command_head_obeys_release_grammar() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let source = r"set x [gets stdin]
[e\x76al $x]";
        assert!(
            dialect_of_code(source, "tcl8.4", "T100").is_empty(),
            "Tcl 8.4 consumes more than two hex digits, so this command is not eval"
        );
        assert!(
            dialect_of_code(source, "tcl8.6", "T100")
                .iter()
                .any(|warning| warning.variable == "x"),
            "Tcl 8.6 consumes two hex digits, so this command is eval"
        );
    }

    #[test]
    fn static_expanded_command_head_preserves_argv_semantics() {
        crate::install_surfaces();
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
        crate::install_surfaces();
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
        crate::install_surfaces();
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
        crate::install_surfaces();
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
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        for (source, code) in [
            ("set x [gets stdin]\n[puts $x]", "T101"),
            ("set x [gets stdin]\n[puts safe; eval $x]", "T100"),
            ("set x [gets stdin]\nprefix[eval $x]", "T100"),
            ("set x [gets stdin]\n\"[puts safe][eval $x]\"", "T100"),
        ] {
            let ws = dialect_of_code(source, D, code);
            assert!(
                ws.iter().any(|warning| warning.variable == "x"),
                "every inner command receives only its own substituted arguments: {source:?}: {ws:?}"
            );
        }
    }

    #[test]
    fn nested_script_sink_distinguishes_deferred_from_literal_arguments() {
        crate::install_surfaces();
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
        crate::install_surfaces();
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
        crate::install_surfaces();
        // Dialect build with a defined `flag`: tclsh errors on the undefined one.
        let ws = dialect_of_code(
            "set flag [expr {rand() < 0.5}]\n\
             if {$flag} {set command eval} else {set command puts}\n\
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
        crate::install_surfaces();
        // Dialect build with a defined `flag`: tclsh errors on the undefined one.
        let ws = dialect_of_code(
            "set flag [expr {rand() < 0.5}]\n\
             if {$flag} {set command eval} else {set command ::eval}\n\
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
        crate::install_surfaces();
        // Tcl requires the namespace before a qualified procedure definition.
        let source = "namespace eval ::ns {}\n\
                      proc ::ns::eval args {}\n\
                      proc ::ns::run {} {set command eval; set line [gets stdin]; $command $line}\n\
                      ::ns::run";
        assert!(
            dialect_of_code(source, D, "T100").is_empty(),
            "a namespace-local proc named eval must not inherit the builtin's sink semantics"
        );
        let control = format!("{source}\nset line [gets stdin]\n::eval $line");
        assert!(
            dialect_of_code(&control, D, "T100")
                .iter()
                .any(|warning| { warning.variable == "line" && warning.sink_command == "eval" })
        );
    }

    #[test]
    fn namespace_shadowed_catch_cannot_hijack_nested_analysis_scaffolding() {
        crate::install_surfaces();
        // Dialect build with the `::ns` namespace: tclsh cannot create `::ns::catch` without it.
        let ws = dialect_of_code(
            "namespace eval ::ns {}\n\
             proc ::ns::catch args {}\n\
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
        crate::install_surfaces();
        // tclsh: `exec ls` returns subprocess stdout.
        let ws = of_code("set output [exec ls]\neval $output", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].variable, "output");
    }

    #[test]
    fn socket_taints() {
        crate::install_surfaces();
        let ws = of_code("set s [socket localhost 80]\neval $s", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn chan_read_taints() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set data [chan read $fd]\neval $data", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn chan_gets_taints() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set data [chan gets $fd]\neval $data", D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn chan_configure_does_not_taint() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: `chan configure $fd` returns channel options, not I/O content —
        // not a taint source.
        assert_eq!(
            channel_codes("set x [chan configure $fd]\neval $x", D),
            Vec::<String>::new()
        );
    }
}

// iRules I/O getters taint their results.
// f5-dialect: HTTP::*, TCP::*, IP::* are not core-tclsh commands.
mod irules_taint_sources {
    use super::super::*;

    #[test]
    fn http_payload_taints() {
        crate::install_surfaces();
        // f5-dialect: HTTP::payload returns the request body.
        let ws = of_code("set body [HTTP::payload]\neval $body", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].variable, "body");
    }

    #[test]
    fn http_header_taints() {
        crate::install_surfaces();
        // f5-dialect.
        let ws = of_code("set hdr [HTTP::header \"Host\"]\neval $hdr", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn http_uri_taints() {
        crate::install_surfaces();
        // f5-dialect.
        let ws = of_code("set uri [HTTP::uri]\neval $uri", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn tcp_payload_taints() {
        crate::install_surfaces();
        // f5-dialect.
        let ws = of_code("set data [TCP::payload]\neval $data", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn ip_client_addr_taints_with_colour() {
        crate::install_surfaces();
        // f5-dialect: IP addresses are tainted but carry the IP_ADDRESS colour.
        let ws = of_code("set addr [IP::client_addr]\neval $addr", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].variable, "addr");
    }
}

// IP/PORT/FQDN/PATH sources suppress T102;
// generic taint does not. f5-dialect sources.
mod source_colour_augmentation {
    use super::super::*;

    #[test]
    fn ip_client_addr_non_dash() {
        crate::install_surfaces();
        assert_eq!(
            of_code("set addr [IP::client_addr]\nregexp $addr test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn tcp_client_port_non_dash() {
        crate::install_surfaces();
        assert_eq!(
            of_code("set port [TCP::client_port]\nregexp $port test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn ssl_sni_non_dash() {
        crate::install_surfaces();
        assert_eq!(
            of_code("set sni [SSL::sni]\nregexp $sni test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn path_prefixed_augments_non_dash() {
        crate::install_surfaces();
        assert_eq!(
            of_code("set uri [HTTP::uri]\nregexp $uri test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn generic_taint_no_augmentation() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set raw [read $fd]\nregexp $raw test", D, "T102");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }
}

// Interpreter special-variable taint sources (`env`, `argv`, `argv0`).
//
// Reading the process environment or the command line is attacker-influenced
// external input, seeded from the dialect-aware special-variable registry
// (`tcl_registry::special_vars`). The restricted iRules interpreter provides
// none of them, so they are sources only in the standard Tcl dialects.
mod special_variable_sources {
    use super::super::*;

    #[test]
    fn env_read_flows_tainted_into_code_sink() {
        crate::install_surfaces();
        let ws = of_code("eval $env(CMD)", D, "T100");
        assert!(!ws.is_empty(), "eval of $env(...) should fire T100");
    }

    #[test]
    fn argv_read_flows_tainted_into_code_sink() {
        crate::install_surfaces();
        let ws = of_code("eval $argv", D, "T100");
        assert!(!ws.is_empty(), "eval of $argv should fire T100");
    }

    #[test]
    fn argv0_read_flows_tainted_into_code_sink() {
        crate::install_surfaces();
        let ws = of_code("eval $argv0", D, "T100");
        assert!(!ws.is_empty(), "eval of $argv0 should fire T100");
    }

    #[test]
    fn local_shadow_is_not_a_source() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // A local `set argv …` shadows the interpreter global: the read sees
        // the clean literal (higher SSA version), so no taint reaches the sink.
        assert_eq!(
            dialect_codes("set argv clean\neval $argv", D),
            Vec::<String>::new()
        );
    }

    #[test]
    fn env_is_not_a_source_in_irules() {
        crate::install_surfaces();
        // The restricted iRules interpreter has no `env`, so it is not seeded
        // as a taint source there.
        assert_eq!(
            of_code("eval $env(CMD)", IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}
