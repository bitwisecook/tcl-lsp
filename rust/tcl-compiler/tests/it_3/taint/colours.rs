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

//! Mitigation colours one by one, and how interpolation and transforms change them.

// PATH_PREFIXED / NON_DASH colours propagate through copies
// and interpolation, keeping later T102 suppressed. f5-dialect sources.
mod taint_colours {
    use super::super::*;

    #[test]
    fn http_uri_getter_has_path_colour() {
        // T100 still fires (dangerous sink regardless of colour).
        let ws = of_code("set uri [HTTP::uri]\neval $uri", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].variable, "uri");
    }

    #[test]
    fn http_path_getter_has_path_colour() {
        let ws = of_code("set p [HTTP::path]\neval $p", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert_eq!(ws[0].variable, "p");
    }

    #[test]
    fn colour_propagates_through_copy() {
        let ws = of_code("set x [HTTP::uri]\nset y $x\neval $y", IR, "T100");
        assert!(ws.iter().any(|w| w.variable == "y"));
    }

    #[test]
    fn non_dash_literal_prefix_stays_t102_safe() {
        // `"prefix${uri}"` no longer starts with `/` but still cannot start with
        // `-` → NON_DASH_PREFIXED, T102 safe.
        assert_eq!(
            of_code(
                "set uri [HTTP::uri]\nset z \"prefix${uri}\"\nregexp $z test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
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
        assert_eq!(
            of_code(
                "set uri [HTTP::uri]\nset z ${uri}/suffix\nregexp $z test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn leading_slash_concat_sets_path_prefixed() {
        // A literal leading `/` keeps the option-injection-safe PATH_PREFIXED.
        assert_eq!(
            of_code(
                "set h [HTTP::header Host]\nset z /${h}\nregexp $z test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// CRLF_FREE colour suppresses IRULE3002 (header) and IRULE3003
// (log). Produced by IP/PORT/FQDN sources and URI::/HTML::encode. f5-dialect.
mod crlf_free {
    use super::super::*;

    #[test]
    fn ip_client_addr_augments_crlf_free() {
        assert_eq!(
            of_code(
                "set addr [IP::client_addr]\nlog local0. $addr",
                IR,
                "IRULE3003"
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn tcp_client_port_augments_crlf_free() {
        assert_eq!(
            of_code(
                "set port [TCP::client_port]\nlog local0. $port",
                IR,
                "IRULE3003"
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn ssl_sni_augments_crlf_free() {
        assert_eq!(
            of_code("set sni [SSL::sni]\nlog local0. $sni", IR, "IRULE3003"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn uri_encode_adds_crlf_free() {
        // f5-dialect: URI::encode strips CR/LF → suppresses IRULE3003.
        assert_eq!(
            of_code(
                "set raw [HTTP::query]\nset enc [URI::encode $raw]\nlog local0. $enc",
                IR,
                "IRULE3003",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn html_encode_adds_crlf_free() {
        assert_eq!(
            of_code(
                "set raw [HTTP::query]\nset enc [HTML::encode $raw]\nlog local0. $enc",
                IR,
                "IRULE3003",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn generic_taint_irule3003_still_warns() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code("set raw [HTTP::query]\nlog local0. $raw", IR, "IRULE3003");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn crlf_free_suppresses_irule3002() {
        assert_eq!(
            of_code(
                "set raw [HTTP::header Host]\nset enc [URI::encode $raw]\nHTTP::header insert X-Fwd $enc",
                IR,
                "IRULE3002",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
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
        assert_eq!(
            of_code(
                "set addr [IP::client_addr]\nset msg \"src=${addr}:80\"\nlog local0. $msg",
                IR,
                "IRULE3003",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn interpolation_without_crlf_preserves() {
        // Interpolation with no literal CR/LF preserves CRLF_FREE →
        // IRULE3003 suppressed.
        // FIXED (lattice-join identity): CRLF_FREE now survives the
        // interpolation `"client:${addr}"` (clean is the join identity), so
        // IRULE3003 stays suppressed.
        assert_eq!(
            of_code(
                "set addr [IP::client_addr]\nset x \"client:${addr}\"\nlog local0. $x",
                IR,
                "IRULE3003",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// SHELL_ATOM augmented from IP/PORT/FQDN; does NOT suppress
// T100. f5-dialect sources.
mod shell_atom {
    use super::super::*;

    #[test]
    fn ip_addr_augments_shell_atom() {
        let ws = of_code("set addr [IP::client_addr]\neval $addr", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn port_augments_shell_atom() {
        let ws = of_code("set port [TCP::client_port]\neval $port", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn fqdn_augments_shell_atom() {
        let ws = of_code("set sni [SSL::sni]\neval $sni", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn shell_atom_lost_at_phi_with_generic() {
        // Both paths tainted → T100 fires regardless of the lost colour.
        let source = "set a [IP::client_addr]\nset g [HTTP::payload]\nif {$cond} {\n    set x $a\n} else {\n    set x $g\n}\neval $x\n";
        assert_ne!(
            of_code(source, IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// [list]/concat produce LIST_CANONICAL, but `eval $lst`
// still fires T100: the tainted first list element becomes the command word.
//
// tclsh (8.6 + 9.0): `proc marker args {puts EXECUTED}; set raw marker;
// set lst [list $raw]; eval $lst` prints `EXECUTED`. LIST_CANONICAL proves
// list-quoting, NOT that the synthesised command word is trusted (D5-T100).
mod list_canonical {
    use super::super::*;

    #[test]
    fn list_command_propagates_taint_to_eval_fires() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: `eval [list $raw]` runs $raw as the command word → T100.
        let ws = channel_of_code(
            "set raw [read $fd]\nset lst [list $raw]\neval $lst",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn lsort_eval_fires() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // tclsh: lsort returns a canonical list but its first element is still
        // user-controlled — no command-word safety.
        let ws = channel_of_code(
            "set raw [read $fd]\nset sorted [lsort $raw]\neval $sorted",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn lrange_eval_fires() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set raw [read $fd]\nset sub [lrange $raw 0 2]\neval $sub",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn split_eval_fires() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set raw [read $fd]\nset parts [split $raw :]\neval $parts",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn list_command_propagates_taint_to_puts() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set raw [read $fd]\nset lst [list $raw]\nputs $lst",
            D,
            "T101",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn concat_of_canonical_lists_eval_fires() {
        // Dialect build with both channels defined: tclsh errors on an undefined
        // `$fd` or `$fd2`.
        let ws = dialect_of_code(
            "set fd stdin\nset fd2 stdin\nset raw [read $fd]\nset a [list $raw]\nset raw2 [read $fd2]\nset b [list $raw2]\nset c [concat $a $b]\neval $c",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn interpolation_preserves_taint_from_list() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set raw [read $fd]\nset lst [list $raw]\nset broken \"prefix $lst\"\neval $broken",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn list_canonical_copy_eval_fires() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // Suppression requires a literal `[list <known-cmd> …]` at the eval site,
        // which is invisible when eval reads a propagated variable → T100 fires.
        let ws = channel_of_code(
            "set raw [read $fd]\nset lst [list $raw]\nset copy $lst\neval $copy",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }

    #[test]
    fn list_canonical_copy_still_tainted() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set raw [read $fd]\nset lst [list $raw]\nset copy $lst\nputs $copy",
            D,
            "T101",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }
}

// A metacharacter-escaping `regsub -all` produces REGEX_LITERAL, lost on
// interpolation, propagated through copies. (Does not suppress T100.)
mod regex_literal {
    use super::super::*;

    /// `regsub -all {[][{}()*+?.\\^$|]} $raw {\\&}`: tclsh 8.6 and 9.0 turn
    /// `a.b\d` into `a\.b\\d`, a pattern matching only itself.
    const QUOTE: &str = "regsub -all {[][{}()*+?.\\\\^$|]} $raw {\\\\&}";

    #[test]
    fn quoting_regsub_produces_colour() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            &format!("set raw [read $fd]\nset safe [{QUOTE}]\neval $safe"),
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn interpolation_invalidates_regex_literal() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            &format!(
                "set raw [read $fd]\nset safe [{QUOTE}]\nset pat \"prefix${{safe}}suffix\"\neval $pat"
            ),
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn regex_literal_propagates_through_copy() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            &format!("set raw [read $fd]\nset safe [{QUOTE}]\nset copy $safe\neval $copy"),
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }
}

// `file normalize` produces PATH_NORMALISED, lost on
// interpolation, propagated through copies. (Does not suppress T100.)
mod path_normalised {
    use super::super::*;

    #[test]
    fn file_normalize_produces_colour() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set raw [read $fd]\nset norm [file normalize $raw]\neval $norm",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn interpolation_invalidates_path_normalised() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set raw [read $fd]\nset norm [file normalize $raw]\nset broken \"${norm}/extra\"\neval $broken",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn path_normalised_propagates_through_copy() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code(
            "set raw [read $fd]\nset norm [file normalize $raw]\nset copy $norm\neval $copy",
            D,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }
}

// HTML::encode / html_encode produce HTML_ESCAPED (+ CRLF_FREE);
// suppress IRULE3001; lost on interpolation; T106 on double-encode. f5-dialect.
mod html_escaped {
    use super::super::*;

    #[test]
    fn html_encode_produces_colour() {
        assert_eq!(
            of_code(
                "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nHTTP::respond 200 content $safe",
                IR,
                "IRULE3001",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn generic_taint_irule3001_fires() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "set raw [HTTP::query]\nHTTP::respond 200 content $raw",
            IR,
            "IRULE3001",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn interpolation_invalidates_html_escaped() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nset broken \"<b>${safe}</b>\"\nHTTP::respond 200 content $broken",
            IR,
            "IRULE3001",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn html_escaped_propagates_through_copy() {
        assert_eq!(
            of_code(
                "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nset copy $safe\nHTTP::respond 200 content $copy",
                IR,
                "IRULE3001",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn html_encode_also_sets_crlf_free() {
        assert_eq!(
            of_code(
                "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nlog local0. $safe",
                IR,
                "IRULE3003",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn html_encode_recognised_as_sanitiser() {
        // f5-dialect: html_encode (portable helper) produces HTML_ESCAPED.
        assert_eq!(
            of_code(
                "set raw [HTTP::query]\nset safe [html_encode $raw]\nHTTP::respond 200 content $safe",
                IR,
                "IRULE3001",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn html_encode_produces_crlf_free() {
        assert_eq!(
            of_code(
                "set raw [HTTP::query]\nset safe [html_encode $raw]\nlog local0. $safe",
                IR,
                "IRULE3003",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn html_encode_double_encode_detected() {
        // T106: html_encode on already HTML_ESCAPED data.
        let ws = of_code(
            "set raw [HTTP::payload]\nset safe [HTML::encode $raw]\nset dup [html_encode $safe]",
            IR,
            "T106",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }
}

// URI::encode / encode_component / escape produce URL_ENCODED
// (+ CRLF_FREE); lost on interpolation; propagated through copies. f5-dialect.
mod url_encoded {
    use super::super::*;

    #[test]
    fn uri_encode_produces_colour() {
        let ws = of_code(
            "set raw [HTTP::payload]\nset enc [URI::encode $raw]\neval $enc",
            IR,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn uri_encode_component_produces_colour() {
        let ws = of_code(
            "set raw [HTTP::payload]\nset enc [URI::encode_component $raw]\neval $enc",
            IR,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn uri_escape_produces_colour() {
        let ws = of_code(
            "set raw [HTTP::payload]\nset enc [URI::escape $raw]\neval $enc",
            IR,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn interpolation_invalidates_url_encoded() {
        let ws = of_code(
            "set raw [HTTP::payload]\nset enc [URI::encode $raw]\nset broken \"prefix${enc}\"\neval $broken",
            IR,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn url_encoded_propagates_through_copy() {
        let ws = of_code(
            "set raw [HTTP::payload]\nset enc [URI::encode $raw]\nset copy $enc\neval $copy",
            IR,
            "T100",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn uri_encode_also_sets_crlf_free() {
        assert_eq!(
            of_code(
                "set raw [HTTP::query]\nset enc [URI::encode $raw]\nlog local0. $enc",
                IR,
                "IRULE3003",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// CRLF_FREE in header/cookie value position suppresses
// IRULE3002; generic taint fires it. f5-dialect.
mod header_token_safe {
    use super::super::*;

    #[test]
    fn crlf_free_in_header_value_suppresses_irule3002() {
        assert_eq!(
            of_code(
                "set val [IP::client_addr]\nHTTP::header insert X-Fwd $val",
                IR,
                "IRULE3002",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn crlf_free_in_cookie_value_suppresses_irule3002() {
        assert_eq!(
            of_code(
                "set val [IP::client_addr]\nHTTP::cookie insert name \"track\" value $val",
                IR,
                "IRULE3002",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn generic_taint_in_header_value_warns() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "set val [HTTP::header Host]\nHTTP::header insert X-Fwd $val",
            IR,
            "IRULE3002",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }
}

// Structural colours stripped on
// interpolation; CRLF_FREE preserved (no literal CRLF). f5-dialect sources.
mod interpolation_colour_invalidation {
    use super::super::*;

    #[test]
    fn list_canonical_stripped() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // `[list [read $fd]]` is tainted (LIST_CANONICAL), the
        // interpolation `"pre $x suf"` strips that colour, and the now-generic
        // tainted `$y` fires T100 at eval.
        let source = "set x [list [read $fd]]\nset y \"pre $x suf\"\neval $y";
        // FIXED: parse_command_substitution now splits args respecting `[...]`
        // nesting, so the `read` source nested inside `[list [read $fd]]` is
        // recovered, `$x`/`$y` are tainted, and T100 fires at `eval $y`.
        assert_ne!(
            channel_of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn path_normalised_stripped() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // `[file normalize [read $fd]]` is tainted; interpolation
        // strips PATH_NORMALISED and T100 fires at eval.
        let source = "set x [file normalize [read $fd]]\nset y \"pre $x\"\neval $y";
        // FIXED: the `read` source nested inside `[file normalize [read $fd]]`
        // is now recovered (nesting-aware arg split), so T100 fires at eval.
        assert_ne!(
            channel_of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn html_escaped_stripped() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let source = "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nset broken \"<b>$safe</b>\"\nHTTP::respond 200 content $broken";
        assert_ne!(
            dialect_of_code(source, IR, "IRULE3001"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn url_encoded_stripped() {
        let source =
            "set raw [HTTP::payload]\nset enc [URI::encode $raw]\nset y \"pre$enc\"\neval $y";
        assert_ne!(
            of_code(source, IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn regex_literal_stripped() {
        let source = "set raw [read $fd]\nset q [regex::quote $raw]\nset y \"^$q\"\neval $y";
        assert_ne!(
            of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn shell_atom_stripped() {
        let source = "set addr [IP::client_addr]\nset y \"host:$addr\"\neval $y";
        assert_ne!(
            of_code(source, IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn crlf_free_preserved_without_literal_crlf() {
        let source = "set addr [IP::client_addr]\nset msg \"client:${addr}\"\nlog local0. $msg";
        // FIXED (lattice-join identity): CRLF_FREE survives the interpolation, so
        // IRULE3003 stays suppressed.
        assert_eq!(
            of_code(source, IR, "IRULE3003"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// Transforms on untainted input stay clean;
// concat of mixed canonicality still propagates taint. f5-dialect sources.
mod transform_colour_edge_cases {
    use super::super::*;

    #[test]
    fn list_with_literal_untainted() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // [list hello] is untainted → eval is clean.
        assert_eq!(
            dialect_of_code("set x [list hello]\neval $x", D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn concat_of_mixed_canonicality() {
        // Dialect build with both channels defined: tclsh errors on an undefined
        // `$fd` or `$fd2`.
        let source = "set fd stdin\nset fd2 stdin\nset raw [read $fd]\nset a [list $raw]\nset b [read $fd2]\nset c [concat $a $b]\neval $c";
        assert_ne!(
            dialect_of_code(source, D, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn uri_encode_with_untainted_no_colour() {
        // f5-dialect: URI::encode on untainted data is still untainted.
        assert_eq!(
            of_code("set enc [URI::encode \"hello\"]\neval $enc", IR, "T100"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn file_normalize_with_untainted_no_colour() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        assert_eq!(
            dialect_of_code(
                "set norm [file normalize \"/tmp/foo\"]\neval $norm",
                D,
                "T100"
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}
