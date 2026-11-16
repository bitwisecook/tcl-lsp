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

//! T102 option injection: positions, suppression by a terminator and values that cannot start with a dash.

// Option injection (T102) — tainted value in option position, no `--`.
//
// tclsh: option-bearing commands (`regexp`, `glob`, …) parse a leading `-` as a
// switch, so a tainted value whose runtime content could start with `-` is an
// option-injection vector unless a `--` terminator ends switch scanning. A
// later *literal* positional also ends scanning (a tainted *subject* after a
// literal pattern is safe). Verified: `regexp -- -foo bar` → `0` (-- ends scan).
mod option_injection {
    use super::super::*;

    #[test]
    fn regexp_tainted_without_terminator() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nregexp $x test", D, "T102");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
        assert!(ws[0].sink_command.contains("regexp"));
    }

    #[test]
    fn regexp_tainted_with_terminator() {
        // tclsh: `--` ends switch scanning, so $x can only be the pattern.
        assert_eq!(
            of_code("set x [read $fd]\nregexp -- $x test", D, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn glob_tainted_without_terminator() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nglob $x", D, "T102");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn glob_tainted_with_terminator() {
        assert_eq!(
            of_code("set x [read $fd]\nglob -- $x", D, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn literal_no_warning() {
        assert_eq!(
            of_code("regexp {^/api} test", D, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn untainted_var_no_warning() {
        assert_eq!(
            of_code("set x \"pattern\"\nregexp $x test", D, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn tainted_subject_after_literal_pattern_no_warning() {
        // tclsh: the literal pattern `{version ([0-9]+)}` is a definite
        // positional that ends switch scanning, so the tainted subject `$x` in a
        // later slot can't be misread as a switch. No T102.
        assert_eq!(
            of_code(
                "set x [read $fd]\nregexp {version ([0-9]+)} $x -> m",
                D,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
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
        assert_eq!(
            of_code(
                "set x [read $fd]\nset y [regsub -all {/\\*.*?\\*/} $x {}]",
                D,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn unset_literal_name_no_warning() {
        // tclsh: `unset name` takes a literal variable name that cannot start
        // with `-`, so even a tainted var by that name is not option-injectable.
        assert_eq!(
            of_code("set thelongname [read $fd]\nunset thelongname", D, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn regexp_tainted_pattern_still_warns() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        // A tainted *pattern* (leading substitution, could expand to `-x`)
        // remains a T102 candidate even when a later positional is literal.
        let ws = channel_of_code("set x [read $fd]\nregexp $x hello", D, "T102");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "x");
        assert!(ws[0].sink_command.contains("regexp"));
    }

    #[test]
    fn switch_after_dash_option_no_warning_for_literal_arg() {
        // `regexp -nocase {ab} $x` — after `-nocase` the literal pattern ends
        // scanning, so the tainted subject is safe.
        assert_eq!(
            of_code("set x [read $fd]\nregexp -nocase {ab} $x", D, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// TestT102Suppression — PATH_PREFIXED / IP / PORT / FQDN colours suppress T102
// (cannot start with `-`); generic taint still fires it. f5-dialect sources.
mod t102_suppression {
    use super::super::*;

    #[test]
    fn http_uri_no_t102() {
        // f5-dialect: HTTP::uri always starts with `/` → no option injection.
        assert_eq!(
            of_code("set uri [HTTP::uri]\nregexp $uri test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn http_path_no_t102() {
        assert_eq!(
            of_code("set p [HTTP::path]\nregexp $p test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn literal_prefix_concat_no_t102() {
        // A fixed non-dash literal prefix is option-injection-safe.
        assert_eq!(
            of_code(
                "set foo \"path_[HTTP::path]\"\nregexp $foo test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn literal_dash_prefix_concat_still_warns() {
        // A fixed `-` prefix is still option-like → warn.
        let ws = of_code("set foo \"-[HTTP::path]\"\nregexp $foo test", IR, "T102");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn generic_taint_still_warns() {
        // Dialect build with a defined channel: tclsh errors on an undefined `$fd`.
        let ws = channel_of_code("set x [read $fd]\nregexp $x test", D, "T102");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn http_header_still_warns() {
        // f5-dialect: HTTP::header is generic-tainted (no path colour) → T102.
        let ws = of_code("set h [HTTP::header Host]\nregexp $h test", IR, "T102");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn path_prefixed_copy_suppresses() {
        assert_eq!(
            of_code(
                "set uri [HTTP::uri]\nset x $uri\nregexp $x test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn ip_client_addr_no_t102() {
        // f5-dialect: IP::client_addr starts with digit/colon → no `-` prefix.
        assert_eq!(
            of_code("set addr [IP::client_addr]\nregexp $addr test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn ip_remote_addr_no_t102() {
        assert_eq!(
            of_code("set addr [IP::remote_addr]\nregexp $addr test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn tcp_client_port_no_t102() {
        // f5-dialect: TCP::client_port is always numeric.
        assert_eq!(
            of_code("set port [TCP::client_port]\nregexp $port test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn tcp_remote_port_no_t102() {
        assert_eq!(
            of_code("set port [TCP::remote_port]\nregexp $port test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn ssl_sni_no_t102() {
        // f5-dialect: SSL::sni is an FQDN — starts with letter/digit, not `-`.
        assert_eq!(
            of_code("set sni [SSL::sni]\nregexp $sni test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn ip_addr_copy_preserves_colour() {
        assert_eq!(
            of_code(
                "set addr [IP::client_addr]\nset x $addr\nregexp $x test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn ip_addr_still_fires_t100() {
        // IP_ADDRESS does NOT suppress T100 — still dangerous for eval.
        let ws = of_code("set addr [IP::client_addr]\neval $addr", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn port_still_fires_t100() {
        let ws = of_code("set port [TCP::client_port]\neval $port", IR, "T100");
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn phi_same_colour_preserves() {
        // Both branches IP_ADDRESS → colour preserved at the phi, no T102.
        let source = "set a [IP::client_addr]\nset b [IP::remote_addr]\nif {1} {\n    set x $a\n} else {\n    set x $b\n}\nregexp $x test\n";
        assert_eq!(
            of_code(source, IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn phi_mixed_colours_preserves_non_dash() {
        // IP_ADDRESS and PORT both augment to NON_DASH_PREFIXED; T102 safe.
        let source = "set a [IP::client_addr]\nset p [TCP::client_port]\nif {$cond} {\n    set x $a\n} else {\n    set x $p\n}\nregexp $x test\n";
        assert_eq!(
            of_code(source, IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn phi_generic_with_coloured_loses() {
        // One branch generic taint, the other IP_ADDRESS → T102 fires.
        let source = "set a [IP::client_addr]\nset g [HTTP::payload]\nif {$cond} {\n    set x $a\n} else {\n    set x $g\n}\nregexp $x test\n";
        assert_ne!(
            of_code(source, IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}

// Leading literal char of an interpolated
// word controls NON_DASH_PREFIXED / PATH_PREFIXED. f5-dialect sources.
mod non_dash_prefixed_interpolation {
    use super::super::*;

    #[test]
    fn literal_alpha_prefix() {
        // `key_${x}` starts with `k` → NON_DASH_PREFIXED.
        assert_eq!(
            of_code(
                "set x [HTTP::header Host]\nset y \"key_${x}\"\nregexp $y test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn literal_digit_prefix() {
        assert_eq!(
            of_code(
                "set x [HTTP::header Host]\nset y \"0${x}\"\nregexp $y test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn literal_slash_prefix() {
        assert_eq!(
            of_code(
                "set x [HTTP::header Host]\nset y \"/${x}\"\nregexp $y test",
                IR,
                "T102",
            ),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn literal_dash_prefix_warns() {
        // `-${x}` starts with `-` → T102 fires.
        let ws = of_code(
            "set x [HTTP::header Host]\nset y \"-${x}\"\nregexp $y test",
            IR,
            "T102",
        );
        assert_ne!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn dynamic_leading_piece_inherits_original() {
        // Lattice-join identity: `${uri}/suffix` inherits PATH_PREFIXED and
        // suppresses T102 — clean is the join identity, so the interpolation
        // does not drop the colour.
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
    fn command_subst_prefix() {
        // `path_[HTTP::path]` starts with `p` → NON_DASH_PREFIXED.
        assert_eq!(
            of_code("set z \"path_[HTTP::path]\"\nregexp $z test", IR, "T102"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }
}
