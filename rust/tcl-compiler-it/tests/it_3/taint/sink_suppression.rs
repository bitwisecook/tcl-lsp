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

//! Which colour silences which sink: the suppression matrix, T100 and T103 mitigation, double encoding and the no-false-positive cases.

// Clean code produces no taint warnings.
//
// tclsh: each snippet is ordinary, side-effect-free Tcl with no attacker
// source, so flagging it would be a false positive.
mod no_false_positives {
    use super::super::*;

    #[test]
    fn literal_set() {
        crate::install_surfaces();
        assert_eq!(codes("set x 42", D), Vec::<String>::new());
    }

    #[test]
    fn incr() {
        crate::install_surfaces();
        assert_eq!(codes("set x 0\nincr x", D), Vec::<String>::new());
    }

    #[test]
    fn safe_eval() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // `eval $x` where x is a literal string — no taint reaches the sink.
        assert_eq!(
            dialect_codes("set x \"puts hello\"\neval $x", D),
            Vec::<String>::new()
        );
    }

    #[test]
    fn safe_expr() {
        crate::install_surfaces();
        assert_eq!(codes("set x 42\nexpr {$x + 1}", D), Vec::<String>::new());
    }

    #[test]
    fn list_operations() {
        crate::install_surfaces();
        assert_eq!(
            codes("set x [list a b c]\nset n [llength $x]", D),
            Vec::<String>::new()
        );
    }
}

// The colour→diagnostic suppression matrix.
// f5-dialect sources.
mod sink_suppression_matrix {
    use super::super::*;

    #[test]
    fn irule3001_not_suppressed_by_crlf_free() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // URL_ENCODED/CRLF_FREE does NOT suppress IRULE3001 (XSS).
        let source =
            "set raw [HTTP::query]\nset enc [URI::encode $raw]\nHTTP::respond 200 content $enc";
        assert_ne!(
            dialect_of_code(source, IR, "IRULE3001"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn irule3001_suppressed_by_html_escaped() {
        crate::install_surfaces();
        let source =
            "set raw [HTTP::query]\nset enc [HTML::encode $raw]\nHTTP::respond 200 content $enc";
        assert_eq!(
            of_code(source, IR, "IRULE3001"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn irule3002_suppressed_by_ip_address() {
        crate::install_surfaces();
        let source = "set addr [IP::client_addr]\nHTTP::header insert X-Client $addr";
        assert_eq!(
            of_code(source, IR, "IRULE3002"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn irule3002_suppressed_by_port() {
        crate::install_surfaces();
        let source = "set port [TCP::client_port]\nHTTP::header insert X-Port $port";
        assert_eq!(
            of_code(source, IR, "IRULE3002"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn irule3003_suppressed_by_fqdn() {
        crate::install_surfaces();
        let source = "set sni [SSL::sni]\nlog local0. $sni";
        assert_eq!(
            of_code(source, IR, "IRULE3003"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn t102_not_suppressed_by_crlf_free() {
        crate::install_surfaces();
        let source = "set raw [HTTP::payload]\nset enc [URI::encode $raw]\nregexp $enc test";
        assert_ne!(
            of_code(source, IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn t102_not_suppressed_by_html_escaped() {
        crate::install_surfaces();
        let source = "set raw [HTTP::payload]\nset enc [HTML::encode $raw]\nregexp $enc test";
        assert_ne!(
            of_code(source, IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn t100_not_suppressed_by_path_prefixed() {
        crate::install_surfaces();
        let source = "set raw [HTTP::uri]\neval $raw";
        assert_ne!(
            of_code(source, IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// TestT100SinkSuppression — exec+SHELL_ATOM and eval+literal-[list-head]
// suppress T100; generic taint and propagated lists do not.
//
// tclsh facts (8.6 + 9.0):
//   * `eval [list puts $raw]` → prints `marker` (puts is the cmd word; $raw is
//     its argument) → suppressed.
//   * `eval [list $raw]`      → prints `EXECUTED` ($raw becomes the cmd word) →
//     fires.
mod t100_sink_suppression {
    use super::super::*;

    #[test]
    fn exec_with_shell_atom_suppressed() {
        crate::install_surfaces();
        // f5-dialect: IP::client_addr → IP_ADDRESS augments to SHELL_ATOM, which
        // suppresses T100 for `exec` (a shell atom can't word-split).
        let source = "set addr [IP::client_addr]\nexec ping $addr";
        // FIXED: emit_sink_warnings now consults the registry's
        // taint_sink_safe_colour — exec's SHELL_ATOM (augmented from IP_ADDRESS)
        // suppresses T100 (an IP atom can't word-split an exec argument).
        assert_eq!(
            of_code(source, IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn exec_with_generic_taint_not_suppressed() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source = "set raw [read $fd]\nexec echo $raw";
        assert_ne!(
            channel_of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn eval_with_list_canonical_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: `eval [list $raw]` runs $raw as the command word.
        let source = "set raw [read $fd]\nset safe [list $raw]\neval $safe";
        let ws = channel_of_code(source, D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn eval_with_generic_taint_not_suppressed() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source = "set raw [read $fd]\neval $raw";
        assert_ne!(
            channel_of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn uplevel_with_list_canonical_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: same hazard as eval — uplevel runs the substituted $raw.
        let source = "set raw [read $fd]\nset safe [list $raw]\nuplevel $safe";
        let ws = channel_of_code(source, D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "uplevel"));
    }

    #[test]
    fn eval_with_literal_list_known_head_suppressed() {
        crate::install_surfaces();
        // tclsh: `eval [list puts $raw]` → prints `marker`; $raw is the ARG, not
        // the cmd word → no code-execution vector → T100 suppressed.
        let source = "set raw [read $fd]\neval [list puts $raw]";
        assert_eq!(
            of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn eval_with_literal_list_tainted_head_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: `eval [list $raw]` → $raw becomes the cmd word → T100 fires.
        let source = "set raw [read $fd]\neval [list $raw]";
        let ws = channel_of_code(source, D, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn uplevel_with_generic_taint_not_suppressed() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source = "set raw [read $fd]\nuplevel $raw";
        assert_ne!(
            channel_of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn subst_not_suppressed_by_shell_atom() {
        crate::install_surfaces();
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // subst is never suppressed by SHELL_ATOM.
        let source = "set addr [IP::client_addr]\nsubst $addr";
        assert_ne!(
            dialect_of_code(source, IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn expr_not_suppressed_by_list_canonical() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let source = "set raw [read $fd]\nset safe [list $raw]\nexpr $safe";
        assert_ne!(
            channel_of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn exec_with_port_suppressed() {
        crate::install_surfaces();
        // f5-dialect: PORT augments to SHELL_ATOM → suppresses exec T100.
        let source = "set port [TCP::client_port]\nexec firewall-cmd $port";
        // FIXED: exec's SHELL_ATOM safe colour (augmented from PORT) now
        // suppresses T100.
        assert_eq!(
            of_code(source, IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// TestT103RegexpPatternInjection — tainted data in a regexp/regsub pattern.
//
// tclsh: the pattern argument of regexp/regsub is compiled as a regex; a tainted
// pattern is a regex-injection / ReDoS vector. The string (subject) position is
// not a pattern → no T103. REGEX_LITERAL (regex::quote) suppresses.
mod t103_regexp_pattern_injection {
    use super::super::*;

    #[test]
    fn tainted_regexp_pattern_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set pat [read $fd]\nregexp $pat teststring", D, "T103");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].code.to_string(), "T103");
    }

    #[test]
    fn tainted_regsub_pattern_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set pat [read $fd]\nregsub $pat teststring replacement",
            D,
            "T103",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn tainted_regexp_with_options_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set pat [read $fd]\nregexp -nocase -- $pat teststring",
            D,
            "T103",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn regex_literal_suppresses_t103() {
        crate::install_surfaces();
        let source = "set raw [read $fd]\nset safe [regex::quote $raw]\nregexp $safe teststring";
        assert_eq!(
            of_code(source, D, "T103"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn braced_regexp_argument_is_not_a_tainted_pattern() {
        crate::install_surfaces();
        assert!(
            of_code("set x [read $fd]\nregexp {$x} literal", D, "T103").is_empty(),
            "a braced dollar spelling is a literal regular expression, not a tainted use"
        );
    }

    #[test]
    fn untainted_regexp_no_warning() {
        crate::install_surfaces();
        assert_eq!(
            of_code("regexp {^[a-z]+$} teststring", D, "T103"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn tainted_non_pattern_arg_no_t103() {
        crate::install_surfaces();
        // tclsh: $input is the subject (string) position, not the pattern.
        assert_eq!(
            of_code("set input [read $fd]\nregexp {^\\d+$} $input", D, "T103"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn tainted_regsub_with_options_fires() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set pat [read $fd]\nregsub -all -nocase -- $pat teststring replacement",
            D,
            "T103",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn t103_message_format() {
        crate::install_surfaces();
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set pat [read $fd]\nregexp $pat teststring", D, "T103");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].message.contains("pat"));
        assert!(ws[0].message.contains("regexp") || ws[0].message.to_lowercase().contains("regex"));
    }
}

// TestT106DoubleEncoding — re-encoding already-encoded data fires T106;
// cross-encode / first-encode / untainted do not. f5-dialect encoders.
mod t106_double_encoding {
    use super::super::*;

    #[test]
    fn double_html_encode() {
        crate::install_surfaces();
        let source = "set raw [HTTP::payload]\nset safe [HTML::encode $raw]\nset double [HTML::encode $safe]";
        let ws = of_code(source, IR, "T106");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].message.contains("HTML-escaped"));
    }

    #[test]
    fn double_uri_encode() {
        crate::install_surfaces();
        let source =
            "set raw [HTTP::payload]\nset enc [URI::encode $raw]\nset double [URI::encode $enc]";
        let ws = of_code(source, IR, "T106");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].message.contains("URL-encoded"));
    }

    #[test]
    fn double_regex_quote() {
        crate::install_surfaces();
        // No dialect catalogues `regex::quote`; the regex-escaping form the
        // registry does catalogue is `regsub` over the metacharacter class
        // (tclsh 8.4.20 to 9.1.0: `regsub -all {[][{}()*+?.\\^$|]} a.b {\\&}`
        // prints `a\.b`, and applied again `a\\\.b`).
        let source = "set raw [HTTP::payload]\n\
            set esc [regsub -all {[][{}()*+?.\\\\^$|]} $raw {\\\\&}]\n\
            set double [regsub -all {[][{}()*+?.\\\\^$|]} $esc {\\\\&}]";
        let ws = of_code(source, IR, "T106");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].message.contains("regex-escaped"));
    }

    #[test]
    fn cross_encode_no_fire() {
        crate::install_surfaces();
        // HTML::encode on URL_ENCODED data is a different colour → no T106.
        let source =
            "set raw [HTTP::payload]\nset enc [URI::encode $raw]\nset html [HTML::encode $enc]";
        assert_eq!(
            of_code(source, IR, "T106"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn first_encode_no_fire() {
        crate::install_surfaces();
        let source = "set raw [HTTP::payload]\nset safe [HTML::encode $raw]";
        assert_eq!(
            of_code(source, IR, "T106"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn double_encode_through_copy() {
        crate::install_surfaces();
        let source = "set raw [HTTP::payload]\nset enc [URI::encode $raw]\nset copy $enc\nset double [URI::encode $copy]";
        assert_ne!(
            of_code(source, IR, "T106"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn double_encode_nested_no_intermediate_fires() {
        crate::install_surfaces();
        // FN fix: `URI::encode [URI::encode $raw]` — the inner encoder's result
        // never lands on a named variable, so the argument-word scan catches
        // it where the named-uses loop could not.
        let source = "set raw [HTTP::payload]\nset double [URI::encode [URI::encode $raw]]";
        assert_ne!(
            of_code(source, IR, "T106"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn single_encode_nested_no_fire() {
        crate::install_surfaces();
        // A single nested encode does not double-encode.
        let source = "set raw [HTTP::payload]\nset once [URI::encode $raw]";
        assert_eq!(
            of_code(source, IR, "T106"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn untainted_no_fire() {
        crate::install_surfaces();
        // Literal data through an encoder does not fire T106.
        let source = "set x [HTML::encode \"hello\"]\nset y [HTML::encode $x]";
        assert_eq!(
            of_code(source, IR, "T106"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn braced_encoder_argument_is_not_a_second_encoding() {
        crate::install_surfaces();
        let source = "set raw [HTTP::payload]\nset x [URI::encode $raw]\nURI::encode {$x}";
        assert!(
            of_code(source, IR, "T106").is_empty(),
            "a braced dollar spelling is literal data, not a second use of the encoded variable"
        );
    }

    #[test]
    fn message_format() {
        crate::install_surfaces();
        let source =
            "set raw [HTTP::payload]\nset enc [HTML::encode $raw]\nset dup [HTML::encode $enc]";
        let ws = of_code(source, IR, "T106");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].message.contains("$enc"));
        assert!(ws[0].message.contains("HTML::encode"));
        assert!(ws[0].message.contains("HTML-escaped"));
    }
}
