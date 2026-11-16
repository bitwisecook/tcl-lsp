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

//! Checks on a command's arguments: option terminators, expected literals, abbreviations, channels, binary formats and addresses.

// W302 — catch without a result variable (silently swallows errors).
//
// tclsh ground truth: the destructive "fire-and-forget" commands below all
// error on a missing target (`after cancel`, `file delete`, `close`, `unset`,
// `dict unset`, `array unset`, `namespace delete`, `rename foo ""`), so
// `catch {…}` with no result var is the documented "do if possible" idiom for
// them — capturing the rc would just yield a discarded value. Constructive
// siblings (`file copy`, `interp create`, `dict set`, `chan configure`,
// `after <ms> <script>`) genuinely shouldn't swallow errors → still fire.
mod catch_ignore {
    use super::super::*;

    #[test]
    fn catch_no_result_is_hint() {
        let ds = of_code("catch {error oops}", D, "W302");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Hint);
    }

    #[test]
    fn catch_with_result_clean() {
        assert!(!fires("catch {error oops} result", D, "W302"));
        assert!(!fires("catch {error oops} result opts", D, "W302"));
    }

    #[test]
    fn fire_and_forget_destructive_commands_suppressed() {
        // Each errors on a missing target in tclsh 9.0 → catch-ignore is the idiom.
        for src in [
            "catch {after cancel $h}",
            "catch {file delete $f}",
            "catch {close $fh}",
            "catch {unset var}",
            "catch {chan close $h}",
            "catch {interp delete slave}",
            "catch {::close $h}", // bare-name normalised
            "catch {dict unset d k}",
            "catch {array unset a *}",
            "catch {namespace delete ::ns}",
            "catch {namespace forget ::pkg::*}",
            "catch {rename foo \"\"}",
        ] {
            assert!(!fires(src, D, "W302"), "unexpected W302 for {src:?}");
        }
    }

    #[test]
    fn genuine_and_constructive_bodies_still_fire() {
        // User proc body / multi-statement / loadable / constructive ensemble
        // subcommands genuinely shouldn't swallow errors.
        for src in [
            "catch {parse_user_input $x}",
            "catch {parse_x; parse_y}",
            "catch {package require Tk}",
            "catch {chan configure $h -buffering none}",
            "catch {file copy a b}",
            "catch {after 1000 puts hi}",
            "catch {interp create slave}",
            "catch {namespace eval ::ns {}}",
            "catch {dict set d k v}",
        ] {
            assert_eq!(count(src, D, "W302"), 1, "expected W302 for {src:?}");
        }
    }
}

// W304 — missing option terminator (--) before a substituted value.
//
// tclsh: option-bearing commands (`regexp`, `regsub`, `exec`, `file delete`,
// `unset`, `load`, …) parse a leading `-` as an option, so `regexp $pat …` with
// `pat="-nocase"` is mis-parsed. Commands that don't support `--`
// (`subst`, `string match`, `lsearch`) are not checked.
mod missing_option_terminator {
    use super::super::*;

    #[test]
    fn regexp_pattern_variable_fires_with_fix() {
        let ds = of_code("regexp $pattern $text", D, "W304");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("--"));
        assert_eq!(ds[0].2.len(), 1);
        assert!(ds[0].2[0].starts_with("-- "));
    }

    #[test]
    fn structurally_safe_pattern_suppressed() {
        // Pattern starts with `(` — cannot be confused with an option.
        assert!(!fires("regexp {(a+)+$} $text", D, "W304"));
    }

    #[test]
    fn literal_dash_pattern_warns() {
        // Pattern starts with `-` → could be read as `-nocase` etc.
        assert_eq!(count("regexp {-[0-9]+} $text", D, "W304"), 1);
    }

    #[test]
    fn with_terminator_clean() {
        assert!(!fires("regexp -- $pattern $text", D, "W304"));
        assert!(!fires("exec -- $cmd", D, "W304"));
        assert!(!fires("file delete -- $path", D, "W304"));
        assert!(!fires("unset -- $varname", D, "W304"));
    }

    #[test]
    fn option_bearing_commands_fire() {
        for src in [
            "regsub $pattern $text X out",
            "exec $cmd",
            "file delete $path",
            "load $fileName",
            "unset $varname",
        ] {
            assert_eq!(count(src, D, "W304"), 1, "expected W304 for {src:?}");
        }
    }

    #[test]
    fn commands_without_dashdash_support_not_checked() {
        // subst / string match / lsearch / glob-literal do not support `--`.
        assert!(!fires("subst $template", D, "W304"));
        assert!(!fires("string match $pattern $value", D, "W304"));
        assert!(!fires("lsearch -exact $domain c", D, "W304"));
        assert!(!fires("glob *.tcl", D, "W304"));
    }

    #[test]
    fn table_and_class_under_irules() {
        assert_eq!(count("table set $key $value", IR, "W304"), 1);
        assert_eq!(
            count("class match $item starts_with my_class", IR, "W304"),
            1
        );
    }
}

// W217 — `unset` whose options consume every argument, so nothing is unset.
//
// `unset` recognises only `-nocomplain` / `--`; any other word (even `-foo`) is
// a variable name. When leading options eat all the arguments, the call is a
// silent no-op — usually a variable named `-nocomplain` that needs a `--`.
mod unset_option_only {
    use super::super::*;

    #[test]
    fn nocomplain_only_fires_with_terminator_fix() {
        let ds = of_code("unset -nocomplain", D, "W217");
        assert_eq!(ds.len(), 1, "expected one W217");
        assert!(ds[0].0.contains("--"), "message mentions --: {:?}", ds[0].0);
        assert_eq!(ds[0].2.len(), 1, "one fix offered");
        assert!(
            ds[0].2[0].starts_with("-- "),
            "fix inserts the terminator: {:?}",
            ds[0].2[0]
        );
    }

    /// Only the first `-nocomplain` and one `--` are options. tclsh 8.4.20,
    /// 8.6.18 and 9.0.4: `set -nocomplain 1; unset -nocomplain -nocomplain;
    /// info exists -nocomplain` is 0, so the second flag is a variable name
    /// and something is unset; `unset --` and `unset -nocomplain --` return
    /// the empty string and unset nothing.
    #[test]
    fn a_terminator_unsets_nothing_and_a_second_flag_is_a_name() {
        for (src, fires) in [
            ("unset --", 1),
            ("unset -nocomplain --", 1),
            ("unset -nocomplain -nocomplain", 0),
        ] {
            assert_eq!(count(src, D, "W217"), fires, "{src}");
        }
    }

    #[test]
    fn option_only_unset_is_not_an_arity_error() {
        // tclsh 8.5+: `unset`, `unset -nocomplain`, `unset --` are all valid
        // no-ops, so the misleading E002 "too few arguments" must not fire —
        // the option-only cases are covered by W217 instead.
        assert!(!fires("unset -nocomplain", D, "E002"));
        assert!(!fires("unset --", D, "E002"));
        assert!(!fires("unset", D, "E002"));
    }

    #[test]
    fn call_that_unsets_a_variable_is_clean() {
        // A real variable name follows the options.
        assert!(!fires("unset -nocomplain $x", D, "W217"));
        assert!(!fires("unset -nocomplain foo", D, "W217"));
        assert!(!fires("unset x", D, "W217"));
        assert!(!fires("unset x y z", D, "W217"));
        // `--` terminator already present, then a real name.
        assert!(!fires("unset -- -nocomplain", D, "W217"));
        // A dash-name in first position is a variable, not an option.
        assert!(!fires("unset -foo", D, "W217"));
        assert!(!fires("unset foo -nocomplain", D, "W217"));
    }
}

// W306 — literal-expected position contains a substitution.
//
// A bare single `$var`/`${ns::v}` regex pattern is the canonical parameterised-
// regex idiom (no braced equivalent) → silent. But `[cmd]` patterns (`[a-z]`
// parses as a *command substitution*, a classic footgun) and quoted patterns
// with a *live* `$var` are flagged.
mod literal_expected {
    use super::super::*;

    #[test]
    fn bare_var_pattern_clean() {
        assert!(!fires("regexp -- $pattern $text", D, "W306"));
        assert!(!fires("regexp -- ${ns::pattern} $text", D, "W306"));
        assert!(!fires("regexp -- {^foo} $text", D, "W306"));
    }

    #[test]
    fn bare_var_pattern_clean_deep_in_proc_body() {
        // Suppression must apply inside nested proc/if/foreach bodies.
        let src = "proc f {} {\n    if {1} {\n        foreach x [list a b] {\n            regexp $pattern $x\n        }\n    }\n}\n";
        assert!(!fires(src, D, "W306"));
    }

    #[test]
    fn command_substitution_pattern_warns() {
        // `[build_re]` / `[a-z]` parse as command substitution → flagged.
        assert_eq!(count("regsub -- [build_re] $text X out", D, "W306"), 1);
        assert_eq!(count("regexp -- [a-z] $text", D, "W306"), 1);
    }

    #[test]
    fn concatenated_var_pattern_no_w306_rust_behaviour() {
        // The analyser only flags a *bracketed* command-substitution /
        // quoted-live-`$var` pattern, not a bare var-concatenation like
        // `$pattern$suffix`, so W306 does NOT fire here. (The reads still
        // surface W210 — `$pattern`/`$suffix`/`$text` are all unset.) Pinned to
        // the actual verdict.
        assert!(!fires("regexp -- $pattern$suffix $text", D, "W306"));
    }

    #[test]
    fn regex_quoting_substitution_pattern_clean() {
        // The wrap T103's quick fix produces. Its whole job is to hand the
        // engine a pattern that matches literally, so it is the remedy rather
        // than the foot-gun — and braces would defeat it by matching the
        // substitution's own source text. Both spellings are exempt.
        assert!(!fires("regexp -- [regex::quote $p] $text", D, "W306"));
        assert!(!fires("regexp -- \"[regex::quote $p]\" $text", D, "W306"));
    }

    #[test]
    fn unproven_command_pattern_still_warns() {
        // The control for the exemption above: it is keyed on the declared
        // REGEX_LITERAL colour, not on the shape, so a command that proves
        // nothing about its result is still flagged.
        assert_eq!(count("regexp -- [build_pattern $p] $text", D, "W306"), 1);
    }

    #[test]
    fn quoted_pure_var_pattern_no_w306() {
        // `"$pattern"` is byte-for-byte identical at runtime to the bare
        // `$pattern` parameterised-pattern idiom (the quotes group nothing), so
        // it is the same canonical form, not a foot-gun — no W306.
        assert!(!fires("regexp -- \"$pattern\" $text", D, "W306"));
        assert!(!fires("regexp -- \"${pattern}\" $text", D, "W306"));
    }

    #[test]
    fn quoted_concatenated_var_pattern_warns() {
        // A live `$suffix` concatenated with literal text (`"pre$suffix"`) is no
        // longer a pure reference — a literal WAS expected — so W306 fires and
        // advises braces.
        let ds = of_code("regexp -- \"pre$suffix\" $text", D, "W306");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("quotes"));
    }

    #[test]
    fn quoted_dollar_anchor_clean_live_var_warns() {
        // `"^foo$"` — the `$` before the closing quote is the regex end-anchor,
        // a *literal* `$` (verified vs tclsh: `regexp -- "^foo$" foo` → 1).
        assert!(!fires("regexp -- \"^foo$\" $text", D, "W306"));
        // `"^foo$bar"` — `$bar` IS a live substitution → still the footgun.
        assert_eq!(count("regexp -- \"^foo$bar\" $text", D, "W306"), 1);
    }

    #[test]
    fn class_match_literal_name_clean_under_irules() {
        assert!(!fires(
            "class match -- [HTTP::uri] ends_with {my_class}",
            IR,
            "W306"
        ));
    }
}

// W307 — non-literal command name.
//
// tclsh: `$cmd arg` dispatches on the *value* of `$cmd` as a command word — a
// command-injection surface when `$cmd` is user-controlled. TclOO/snit object
// dispatch (`$obj method`) and param-dispatcher idioms are recognised and
// suppressed.
//
// `$cmd` in a `foreach` over a list of *known* command names is not suppressed
// here — that known-command-list provenance is absent on this surface, so W307
// fires there (pinned below).
mod non_literal_command {
    use super::super::*;

    #[test]
    fn variable_and_command_subst_command_name_fire() {
        let dv = of_code("$cmd arg1 arg2", D, "W307");
        assert_eq!(dv.len(), 1);
        assert!(dv[0].0.contains("Non-literal"));
        assert_eq!(dv[0].1, Severity::Warning);
        assert_eq!(count("[get_cmd] arg1", D, "W307"), 1);
    }

    #[test]
    fn literal_command_clean() {
        assert!(!fires("puts hello", D, "W307"));
    }

    #[test]
    fn tcloo_instance_dispatch_clean() {
        // `$obj greet world` where obj is a TclOO instance is method dispatch.
        let src = "oo::class create MyClass {\n    method greet {name} {\n        puts \"hello $name\"\n    }\n}\nset obj [MyClass new]\n$obj greet world\n";
        assert!(!fires(src, D, "W307"));
    }

    #[test]
    fn param_used_as_dispatcher_clean() {
        // A proc dispatching on its own parameter documents an object-handle API.
        assert!(!fires("proc once {tree} { $tree visit }", D, "W307"));
        let walk = "proc walk {tree} {\n    foreach n [$tree leaves] {\n        $tree visit $n\n    }\n}\n";
        assert!(!fires(walk, D, "W307"));
    }

    #[test]
    fn local_dispatcher_not_param_still_fires() {
        // A single dispatch on a non-param local is still flagged.
        assert_eq!(
            count(
                "proc f {} {\n    set foo [some_factory]\n    $foo method\n}",
                D,
                "W307"
            ),
            1
        );
    }

    #[test]
    fn foreach_over_known_commands_is_suppressed() {
        // `foreach cmd [list …]` now folds the literal list into a CONSTSET, so
        // `$cmd` dispatch resolves to each element.  When every element is a
        // *known* command (`puts`/`set`/`expr`) the dispatch is provably safe and
        // W307 is suppressed.
        let known = "foreach cmd [list puts set expr] {\n    $cmd hello\n}\n";
        assert!(
            !fires(known, D, "W307"),
            "known-command-list dispatch must not fire W307 (folded provenance)"
        );
        // A list whose elements are *unresolved* command names is still flagged —
        // the folded names don't resolve to real commands, so the dispatch may
        // execute anything.
        let unknown = "foreach cmd [list unknown_cmd_a unknown_cmd_b] {\n    $cmd value\n}\n";
        assert!(fires(unknown, D, "W307"));
    }
}

// W213 — unset without -nocomplain on a possibly-undefined variable.
//
// tclsh ground truth: `unset x` on a never-set var → `can't unset "x": no such
// variable`. `unset -nocomplain` opts out (tclsh suppresses the error too);
// unsetting a previously-set var is fine.
mod unset_nocomplain {
    use super::super::*;

    /// A document's global frame is open: another file or the host may have
    /// set a global, so a file-level `unset x` with no earlier write here
    /// reports nothing. It is closed for what the document itself proves, and
    /// a procedure's locals are closed. tclsh 8.4.20 to 9.1.0 raise `can't
    /// unset "x": no such variable` for the procedure form and for the second
    /// file-level `unset` on every run.
    #[test]
    fn unset_of_an_unbound_name_reports_where_the_document_proves_it() {
        for (src, fires) in [
            ("unset x", 0),
            ("proc f {} {unset x}", 1),
            ("set x 1\nunset x\nunset x", 1),
        ] {
            let ds = of_code(src, D, "W213");
            assert_eq!(ds.len(), fires, "{src}: {ds:?}");
            for (message, _, _) in &ds {
                assert!(
                    message.contains('x') && message.contains("nocomplain"),
                    "{src}: {message}"
                );
            }
        }
    }

    #[test]
    fn nocomplain_and_defined_var_clean() {
        assert!(!fires("unset -nocomplain x", D, "W213"));
        assert!(!fires("set x 1\nunset x", D, "W213"));
        assert!(!fires("unset -nocomplain -- x", D, "W213"));
    }
}

// W121 — invalid subnet mask (non-contiguous bits).
//
// Heuristic over dotted-quad literals: `255.255.255.1` looks like a mask but has
// non-contiguous one-bits. Valid CIDR masks (/0…/32) are clean.
mod invalid_subnet_mask {
    use super::super::*;

    #[test]
    fn non_contiguous_masks_fire() {
        let ds = of_code("set mask 255.255.255.1", D, "W121");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("non-contiguous"));
        for m in ["255.0.255.0", "255.255.0.128", "255.255.253.0"] {
            assert_eq!(
                count(&format!("set mask {m}"), D, "W121"),
                1,
                "expected W121 for {m}"
            );
        }
    }

    #[test]
    fn valid_masks_clean() {
        for m in [
            "255.255.255.0",
            "255.255.254.0",
            "255.255.255.192",
            "255.255.255.255",
            "255.0.0.0",
            "0.0.0.0",
            "128.0.0.0",
        ] {
            assert!(
                !fires(&format!("set mask {m}"), D, "W121"),
                "unexpected W121 for {m}"
            );
        }
    }

    #[test]
    fn regular_ips_not_masks() {
        assert!(!fires("set addr 10.0.0.1", D, "W121"));
        assert!(!fires("set addr 192.168.1.1", D, "W121"));
    }

    #[test]
    fn invalid_mask_in_string_still_fires_with_suggestion() {
        // The invalid mask inside a string literal still fires; the suggested
        // mask is in the message. The diagnostic carries the suggestion in the
        // message text but no structured fix object here, so assert on the message.
        let ds = of_code("puts \"mask is 255.255.255.1\"", D, "W121");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("255.255.255.0"));
    }
}

// W124 — mistyped IPv4 address.
//
// tclsh: `192.168.1.256` is not a valid IP (octet > 255). The SSA-traced
// **W124** check covers both halves of what the lexical **W122** used to
// describe (octet > 255 and a leading-zero octal ambiguity); W122 duplicated
// it with no independent producer and was retired. `ip_count`
// keeps summing "W122 OR W124" (W122's contribution is now always 0) rather
// than being narrowed to W124 alone, so a future accidental reintroduction of
// a W122 producer would still be counted here rather than silently going
// unnoticed by this suite. Octet > 255 → Error; octal-ambiguous leading zero
// → Warning. OIDs / version numbers are excluded.
mod mistyped_ipv4 {
    use super::super::*;

    /// Count of W122 (retired, always 0 — see the module note) + W124.
    fn ip_count(src: &str) -> usize {
        count(src, D, "W122") + count(src, D, "W124")
    }

    #[test]
    fn octet_over_255_is_error() {
        assert_eq!(ip_count("set addr 192.168.1.256"), 1);
        let ds = of_code("set addr 192.168.1.256", D, "W124");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("exceeds 255"));
        assert_eq!(ds[0].1, Severity::Error);
        assert_eq!(ip_count("set addr 10.0.0.999"), 1);
    }

    #[test]
    fn octal_ambiguous_leading_zero_is_warning() {
        let ds = of_code("set addr 192.168.01.1", D, "W124");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("leading zero"));
        assert_eq!(ds[0].1, Severity::Warning);
        assert_eq!(ip_count("set addr 192.168.077.1"), 1);
        assert_eq!(ip_count("set addr 10.00.0.1"), 1);
    }

    #[test]
    fn valid_ips_clean() {
        for a in ["192.168.1.1", "10.0.0.1", "0.0.0.0", "255.255.255.0"] {
            assert_eq!(
                ip_count(&format!("set addr {a}")),
                0,
                "unexpected IP diag for {a}"
            );
        }
    }

    #[test]
    fn non_octal_leading_zero_clean() {
        // A leading zero followed by a non-octal digit is not octal-ambiguous.
        for a in ["192.168.09.1", "10.0.08.1", "192.168.099.1"] {
            assert_eq!(
                ip_count(&format!("set addr {a}")),
                0,
                "unexpected IP diag for {a}"
            );
        }
    }

    #[test]
    fn oids_and_version_numbers_excluded() {
        // LDAP / SNMP OIDs and User-Agent version numbers are not IPv4 quads.
        assert_eq!(ip_count("set oid 1.3.6.1.4.1.4203.1.11.3"), 0);
        assert_eq!(ip_count("set oid 1.3.6.1.2.1.1.5.0"), 0);
        assert_eq!(
            ip_count("set ua \"Mozilla/5.0 Chrome/28.0.1550.0 Safari/537.36\""),
            0
        );
    }

    #[test]
    fn ssa_traced_in_proc_and_top_level() {
        // W124 covers proc bodies and top-level alike.
        let ds = of_code("proc f {} { set a 192.168.1.256 }", D, "W124");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("exceeds 255"));
        assert_eq!(count("set a 192.168.1.256", D, "W124"), 1);
        // A valid IP / version string inside a proc stays clean.
        assert!(!fires("proc f {} { set a 10.0.0.1 }", D, "W124"));
        assert!(!fires(
            "proc f {} { set ua \"Chrome/28.0.1550.0\" }",
            D,
            "W124"
        ));
    }
}

// W126 — channel-type validation.
//
// SCCP-typed: a value whose inferred type cannot be a channel handle
// (INT/LIST) passed to a channel slot (`close`/`gets`/…), or a non-standard
// string literal, is flagged. Channels from open/socket, stdout/stderr, opaque
// STRING-typed handles, and conservatively-typed params are clean.
mod channel_validation {
    use super::super::*;

    #[test]
    fn typed_channels_and_std_channels_clean() {
        assert!(!fires(
            "proc f {} { set fd [open /tmp/t r]; gets $fd line; close $fd }",
            D,
            "W126"
        ));
        assert!(!fires(
            "proc f {} { set s [socket localhost 80]; close $s }",
            D,
            "W126"
        ));
        assert!(!fires("proc f {} { puts stdout hello }", D, "W126"));
        assert!(!fires("proc f {} { puts stderr hello }", D, "W126"));
    }

    #[test]
    fn int_and_list_typed_values_fire() {
        let di = of_code("proc f {} { set x 42; close $x }", D, "W126");
        assert_eq!(di.len(), 1);
        assert!(di[0].0.contains("INT"));
        let dl = of_code("proc f {} { set l [list a b]; close $l }", D, "W126");
        assert_eq!(dl.len(), 1);
        assert!(dl[0].0.contains("LIST"));
    }

    #[test]
    fn non_channel_string_literal_fires() {
        let ds = of_code("proc f {} { gets notachannel line }", D, "W126");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("notachannel"));
    }

    #[test]
    fn opaque_param_and_string_typed_clean() {
        // Conservative: a bare param / round-tripped STRING handle could be a channel.
        assert!(!fires("proc f {fd} { close $fd }", D, "W126"));
        assert!(!fires(
            "proc f {token} { upvar 0 $token state; set sock $state(sock); close $sock }",
            D,
            "W126"
        ));
    }
}

// W200 — binary format signed/unsigned modifier requires Tcl 8.5+ (TIP 275).
//
// Version-gated on the dialect's *effective Tcl version* (the §6
// argument-DSL rung): it warns under 8.4-based runtimes (tcl8.4, and the
// whole F5 trunk — iRules, iApps and tmsh all ride the fork of Tcl at
// 8.4.6, measured in bigip-irule-parser-measurements.md §4a) and is clean
// from 8.5 up.
mod binary_format_modifiers {
    use super::super::*;

    #[test]
    fn unsigned_modifier_warns_under_old_dialects() {
        let ds = of_code("binary format su $val", "tcl8.4", "W200");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("modifier"));
        assert_eq!(ds[0].1, Severity::Warning);
        assert_eq!(count("binary scan $data su x", "tcl8.4", "W200"), 1);
        assert_eq!(count("binary format su $val", IR, "W200"), 1);
        // iApps ride the `f5-tcl` trunk (fork of Tcl at 8.4.6 — measured,
        // bigip-irule-parser-measurements.md §4a: every 8.5 discriminator
        // fails), so the 8.5 u/s modifiers flag there too.
        assert_eq!(count("binary format su $val", "f5-iapps", "W200"), 1);
    }

    #[test]
    fn unsigned_modifier_clean_under_tcl86() {
        assert!(!fires("binary format su $val", "tcl8.6", "W200"));
    }

    #[test]
    fn no_modifier_clean() {
        assert!(!fires("binary format s $val", "tcl8.4", "W200"));
    }

    // TIP 275 added only the `u` suffix. `s` is the short-integer specifier,
    // so `ss` is two 2-byte fields on every release — verified on tclsh
    // 8.4.20, 8.5.19, 8.6.18 and 9.0.4 — and never a signedness modifier.
    #[test]
    fn short_specifier_is_not_a_modifier() {
        assert!(!fires("binary format ss 1 2", "tcl8.4", "W200"));
        assert!(!fires("binary scan $x ss a b", "tcl8.4", "W200"));
        assert!(!fires("binary format is $val $v2", "tcl8.4", "W200"));
        // The real `u` modifier is unaffected.
        assert_eq!(count("binary format su 1", "tcl8.4", "W200"), 1);
    }

    // `GetFormatSpec` in tclBinary.c consumes a `u` after the command
    // character without consulting the type, so the gate is not restricted to
    // the integer fields: `binary format au 1` is `bad field specifier "u"` on
    // tclsh 8.4.20 and clean on 8.5.19.
    #[test]
    fn unsigned_modifier_warns_after_any_field_letter() {
        assert_eq!(count("binary format au 1", "tcl8.4", "W200"), 1);
        assert_eq!(count("binary format du 1.0", "tcl8.4", "W200"), 1);
        assert!(!fires("binary format au 1", "tcl8.6", "W200"));
    }

    // `t` is a real 8.5 field letter and `T` is not a field letter on any
    // release, so the gate follows the shared grammar rather than a private
    // table that had them the wrong way round.
    #[test]
    fn gate_follows_the_shared_field_letters() {
        assert_eq!(count("binary format tu 1", "tcl8.4", "W200"), 1);
        assert!(!fires("binary format Tu 1", "tcl8.4", "W200"));
    }

    // Every field shares the format token's span and the gate dedupes by
    // span, so a template with several gated fields is one squiggle.
    #[test]
    fn one_diagnostic_per_format_string() {
        assert_eq!(
            count("binary format cu1su1iu1 $a $b $c", "tcl8.4", "W200"),
            1
        );
    }
}

// W202 — a `binary format` / `binary scan` field letter that postdates the
// target. `t n m r R q Q` arrive in Tcl 8.5 and are `bad field specifier` on
// tclsh 8.4.20, for both subcommands (they share `GetFormatSpec`). Separate
// from W200 because the fix differs: a suffix can be dropped, an absent
// letter needs a different field.
mod binary_field_letters {
    use super::super::*;

    #[test]
    fn tcl85_only_letters_warn_under_old_dialects() {
        for letter in ["t", "n", "m", "r", "R", "q", "Q"] {
            let src = format!("binary format {letter} 1");
            let ds = of_code(&src, "tcl8.4", "W202");
            assert_eq!(ds.len(), 1, "{letter} should be gated on 8.4");
            assert_eq!(ds[0].1, Severity::Warning);
            assert!(ds[0].0.contains(letter), "{letter}: {}", ds[0].0);
        }
        assert_eq!(count("binary scan $d q v", "tcl8.4", "W202"), 1);
        // The F5 trunk forks Tcl at 8.4.6, so it gates too.
        assert_eq!(count("binary format q 1.0", IR, "W202"), 1);
        assert_eq!(count("binary format q 1.0", "f5-iapps", "W202"), 1);
    }

    #[test]
    fn tcl85_only_letters_clean_from_85_up() {
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
            assert!(!fires("binary format q 1.0", dialect, "W202"));
        }
    }

    #[test]
    fn letters_on_every_release_stay_clean() {
        for letter in [
            "a", "A", "b", "B", "h", "H", "c", "s", "S", "i", "I", "w", "W", "f", "d", "x", "X",
        ] {
            let src = format!("binary format {letter} 1");
            assert!(!fires(&src, "tcl8.4", "W202"), "{letter} exists on 8.4");
        }
    }

    // A `package require Tcl 8.5` raises the effective version, which is the
    // documented fix.
    #[test]
    fn package_require_raises_the_floor() {
        assert!(!fires(
            "package require Tcl 8.5\nbinary format q 1.0",
            "tcl8.4",
            "W202",
        ));
    }

    // One squiggle per template per code, and the two codes are independent.
    #[test]
    fn one_diagnostic_per_template_and_codes_are_independent() {
        assert_eq!(count("binary format qrm 1 2 3", "tcl8.4", "W202"), 1);
        assert!(!fires("binary format q 1.0", "tcl8.4", "W200"));
        assert!(!fires("binary format cu 1", "tcl8.4", "W202"));
        // A gated letter carrying a gated suffix earns both, once each.
        assert_eq!(count("binary format qu 1.0", "tcl8.4", "W200"), 1);
        assert_eq!(count("binary format qu 1.0", "tcl8.4", "W202"), 1);
    }

    // Site selection runs through the registry's `FormatType::Binary`
    // metadata and the head's effective command identity, not the spelling:
    // `::binary` is the builtin, and a `proc binary` that takes the name
    // over is not.
    #[test]
    fn site_selection_follows_command_identity() {
        assert_eq!(count("::binary format q 1.0", "tcl8.4", "W202"), 1);
        assert_eq!(count("::binary format qu 1.0", "tcl8.4", "W200"), 1);
        // A user-defined `binary` is not the builtin, so its arguments are
        // not a binary template.
        assert!(!fires(
            "proc binary {a b} {}\nbinary format q 1.0",
            "tcl8.4",
            "W202",
        ));
    }

    // An expanded word makes the whole argument layout unknown at analysis
    // time — `{*}$sub` can supply the subcommand, the template, or both — so
    // the gate abstains rather than reading a fixed position.
    #[test]
    fn expanded_words_abstain() {
        assert!(!fires("binary {*}$sub q 1.0", "tcl8.4", "W202"));
        assert!(!fires("binary format {*}$w q 1.0", "tcl8.4", "W202"));
        assert!(!fires("binary {*}[list format] q 1.0", "tcl8.4", "W202"));
        // The unexpanded form of the same call is still checked.
        assert_eq!(count("binary format q 1.0", "tcl8.4", "W202"), 1);
    }

    // A dynamic template has no literal text to read. Without the guard the
    // scanner reads the *variable name*: `$fmt` carries `f`, `m` and `t`, so
    // the gated `m`/`t` fired on every `binary format $fmt ...` under 8.4.
    // A compound word is dynamic too — `"a$fmt"` is several tokens whose
    // representative is an ordinary `Esc`, so a kind-only check missed it.
    #[test]
    fn dynamic_template_is_not_checked() {
        assert!(!fires("binary format \"a$fmt\" 1", "tcl8.4", "W202"));
        assert!(!fires("binary format \"a${fmt}b\" 1", "tcl8.4", "W202"));
        assert!(!fires("binary format \"q$x\" 1", "tcl8.4", "W202"));
        assert!(!fires("binary format a[fn] 1", "tcl8.4", "W202"));
        assert!(!fires("binary format \"a$au\" 1", "tcl8.4", "W200"));
        assert!(!fires("binary format $fmt 1", "tcl8.4", "W202"));
        assert!(!fires("binary scan $d $fmt v", "tcl8.4", "W202"));
        assert!(!fires("binary format [get_fmt] 1", "tcl8.4", "W202"));
        // Same guard, W200's side: `$au` read as a field plus a gated `u`.
        assert!(!fires("binary format $au 1", "tcl8.4", "W200"));
        assert!(!fires("binary format $fmt 1", "tcl8.4", "W200"));
        // A braced literal is still a literal and still checked.
        assert_eq!(count("binary format {q} 1.0", "tcl8.4", "W202"), 1);
    }
}

// W311 — unsafe channel encoding mismatch.
//
// `-encoding binary` with a non-binary `-translation` (auto/crlf) corrupts
// binary data. Matching `-translation binary` (or no translation) is clean.
mod encoding_mismatch {
    use super::super::*;

    #[test]
    fn binary_encoding_text_translation_warns() {
        let ds = of_code(
            "fconfigure $fd -encoding binary -translation auto",
            D,
            "W311",
        );
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
        assert!(ds[0].0.to_lowercase().contains("binary"));
        assert_eq!(
            count(
                "chan configure $fd -encoding binary -translation crlf",
                D,
                "W311"
            ),
            1
        );
    }

    #[test]
    fn matching_or_absent_translation_clean() {
        assert!(!fires(
            "fconfigure $fd -encoding binary -translation binary",
            D,
            "W311"
        ));
        assert!(!fires(
            "fconfigure $fd -encoding utf-8 -translation auto",
            D,
            "W311"
        ));
        assert!(!fires("fconfigure $fd -encoding binary", D, "W311"));
    }
}

// W145 — ambiguous keyword abbreviation.
//
// tclsh ground truth (8.6.16):
//   `string l abc`   → unknown or ambiguous subcommand "l": must be
//                      bytelength, cat, compare, equal, first, index, is,
//                      last, length, …
//   `string le abc`  → 3            (unique prefix of `length`)
//   `string trim " x "` → x         (exact spelling beats the prefix it shares)
//   `namespace ensemble create -command ::e -subcommands {alpha} -prefixes 0`
//   then `::e al`    → unknown subcommand "al": must be alpha
mod ambiguous_abbreviation {
    use super::super::*;

    #[test]
    fn an_ambiguous_subcommand_prefix_fires_and_names_the_candidates() {
        let ds = of_code("string l abc", D, "W145");
        assert_eq!(ds.len(), 1, "{:?}", codes("string l abc", D));
        assert!(ds[0].0.contains("'l'"), "{:?}", ds[0].0);
        assert!(ds[0].0.contains("'last'"), "{:?}", ds[0].0);
        assert!(ds[0].0.contains("'length'"), "{:?}", ds[0].0);
        // The ambiguity diagnostic replaces the unknown-subcommand guess.
        assert!(!fires("string l abc", D, "W001"));
    }

    #[test]
    fn one_quick_fix_per_candidate_all_requiring_review() {
        let fixes = Analyser::new()
            .analyse("string l abc", D)
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W145")
            .flat_map(|d| d.fixes.clone())
            .collect::<Vec<_>>();
        assert!(fixes.len() >= 2, "{fixes:?}");
        let texts: Vec<&str> = fixes.iter().map(|f| f.new_text.as_str()).collect();
        assert!(texts.contains(&"last"), "{texts:?}");
        assert!(texts.contains(&"length"), "{texts:?}");
        for fix in &fixes {
            assert_eq!(
                fix.safety,
                tcl_compiler::irules_checks::FixSafety::RequiresReview,
                "an ambiguity fix is a manual pick, never auto-applied"
            );
        }
    }

    #[test]
    fn unique_prefixes_and_exact_spellings_stay_clean() {
        // TN: unique prefix, full spelling, and an exact word that is also a
        // proper prefix of longer siblings.
        for src in [
            "string le $s",
            "string length $s",
            "string trim $s",
            "string last a abc",
            "info exists x",
            "info ex x",
        ] {
            assert!(!fires(src, D, "W145"), "{src}: {:?}", codes(src, D));
        }
    }

    #[test]
    fn an_ambiguous_option_prefix_fires() {
        let src = "lsearch -a {a b} a";
        let ds = of_code(src, D, "W145");
        assert_eq!(ds.len(), 1, "{:?}", codes(src, D));
        assert!(ds[0].0.contains("'-all'"), "{:?}", ds[0].0);
        assert!(ds[0].0.contains("'-ascii'"), "{:?}", ds[0].0);
    }

    #[test]
    fn a_unique_option_prefix_stays_clean() {
        assert!(!fires("lsearch -noc {a b} a", D, "W145"));
        assert!(!fires("lsearch -nocase {a b} a", D, "W145"));
    }

    #[test]
    fn dynamic_and_expanded_words_abstain() {
        for src in [
            "string $sub abc",
            "string [pick] abc",
            "string {*}$words abc",
            "lsearch {*}$opts {a b} a",
        ] {
            assert!(!fires(src, D, "W145"), "{src}: {:?}", codes(src, D));
        }
    }

    #[test]
    fn a_prefixes_0_ensemble_abstains() {
        // The file turns prefix matching off for this ensemble, so `al` is a
        // plain unknown subcommand — never an ambiguity.
        let src = "namespace eval ::e {\n\
                   proc alpha {} { return 1 }\n\
                   proc alias {} { return 2 }\n\
                   namespace ensemble create -command ::e -subcommands {alpha alias} -prefixes 0\n\
                   }\n\
                   ::e al\n";
        assert!(!fires(src, D, "W145"), "{:?}", codes(src, D));
    }

    #[test]
    fn a_data_word_that_happens_to_be_a_prefix_is_not_a_keyword_position() {
        // FP guard: the ambiguous-looking word sits in a value position, not
        // a keyword one.
        for src in ["set l abc", "puts l", "lappend xs l"] {
            assert!(!fires(src, D, "W145"), "{src}: {:?}", codes(src, D));
        }
    }
}
