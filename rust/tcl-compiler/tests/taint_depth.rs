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

//! Depth coverage for `tcl_compiler::taint` — the *breadth* the core
//! `taint.rs` (291 tests) leaves thin: the full sink-family matrix
//! (T100–T106, IRULE3001–3004 / 3101), every taint SOURCE namespace and its
//! source colour, every COLOUR transform/augmentation, the position-aware sink
//! filters (`puts` channel slot, `socket` network-address slots, `[list
//! <known-cmd>]` head recognition), the W313 destructive-file pass, and the
//! double-encode / setter-constraint edges.
//!
//! This file deliberately reuses the `taint.rs` harness *shape* —
//! [`warns`] / [`codes`] / [`of_code`] over [`find_taint_warnings_for_cu`],
//! with the same [`D`] (`tcl8.6`) / [`IR`] (`f5-irules`) dialect constants — but
//! every `#[test]` here is NEW: it exercises a sink/source/colour/code path or a
//! position-aware filter that `taint.rs` does not already pin (verified by
//! reading that file's module list before authoring). Where it touches an
//! already-covered code it does so through a *different* command, dialect, or
//! suppression branch.
//!
//! ## Policy-vs-semantic proof split (C-Tcl)
//!
//! Taint is a *security/policy* analysis, not a runtime value: tclsh implements
//! no taint, so a verdict like "this sink is flagged" / "this colour suppresses
//! that code" has no tclsh ground truth and is asserted as a pure policy check.
//! Where a test's *premise* is a Tcl-semantic fact — a command's actual return,
//! a string transform, whether the first list element becomes the command word —
//! it is verified against `scripts/dev/tclsh_check.sh` (tclsh8.6 + tclsh9.0) and
//! cited in a `// tclsh:` comment. The headline facts proven while authoring
//! this file (all agree on 8.6 and 9.0):
//!   * `file join /base sub ../etc` → `/base/sub/../etc` — a portable *concat*
//!     that does NOT canonicalise away `..` (so `[file join]` earns `PATH_JOINED`,
//!     not `PATH_NORMALISED`, and does not clear W313).
//!   * `regsub -all a banana X` → `bXnXnX` — `regsub` returns the substituted
//!     *string* (a content value, not a count), so a tainted pattern is a live
//!     regex-injection vector (T103).
//!   * `eval [split $raw " "]` with `set raw "marker x"` runs `marker` — the
//!     first split element becomes the command word, so list-shaped taint still
//!     reaches T100 at `eval`.
//!   * `interp eval $i {expr 6*7}` → `42` — `interp eval` executes its script in
//!     the child interpreter (the T105 cross-interp premise).
//!   * `gets $fd line` (WITH varName) → returns the byte *count* (`2`), writing
//!     the line into `$line` — the count-return form, distinct from the
//!     content-returning `gets $fd`.
//!   * `regexp -start 2 a xaxa` → `1` — `-start` consumes its value argument, so
//!     the pattern index sits after it.
//!   * `subst {val=$x sum=[expr 2+3]}` → `val=5 sum=5` — `subst` evaluates
//!     embedded `$var` / `[cmd]`, the code-execution premise behind its T100.
//!
//! F5/iRules commands (`HTTP::*`, `IP::*`, `SSL::*`, `URI::*`, `HSL::*`,
//! `connect`, …) are NOT core-tclsh commands, so snippets that mention them
//! carry a `// f5-dialect` note — there is no tclsh proof for the command
//! itself, only the policy verdict. iRules *sources* taint under plain Tcl too
//! (they are registered globally), so generic T100–T106 source+sink tests use
//! [`D`]; iRules *sinks* (IRULE3001–3004 / 3101) only classify under [`IR`].

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::taint::{TaintColour, TaintLattice, TaintWarning, find_taint_warnings_for_cu};
use tcl_registry::registry_for_dialect;

/// Default dialect for dialect-insensitive snippets. iRules sources still taint
/// here (registered globally), so the generic T100–T106 tests use it.
const D: &str = "tcl8.6";
/// iRules dialect — gates the IRULE3001–3004 / IRULE3101 sinks and `log` →
/// IRULE3003.
const IR: &str = "f5-irules";

/// Every `TaintWarning` the whole-unit taint pass surfaces for `src` under
/// `dialect`.
fn warns(src: &str, dialect: &str) -> Vec<TaintWarning> {
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    find_taint_warnings_for_cu(&cu, registry, dialect_opt)
}

/// Sorted list of diagnostic codes.
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

/// True when any warning of `code` names `var`.
fn has(src: &str, dialect: &str, code: &str, var: &str) -> bool {
    of_code(src, dialect, code)
        .iter()
        .any(|w| w.variable == var)
}

// ===========================================================================
// Colour-mask invariants — the public `TaintColour` masks that drive every
// suppression decision. Pure lattice algebra (no tclsh analogue): asserts the
// exact membership of REDIRECT_SAFE / CRLF_SAFE / T102_SAFE / ALL so a future
// mask edit that silently widens a suppression set is caught here.
// ===========================================================================
mod colour_masks {
    use super::*;

    #[test]
    fn redirect_safe_is_path_prefixed_or_normalised() {
        // IRULE3004 (open-redirect) clears only on a same-origin proof: a `/`
        // prefix or a `[file normalize]`d path.
        assert!(TaintColour::REDIRECT_SAFE.contains(TaintColour::PATH_PREFIXED));
        assert!(TaintColour::REDIRECT_SAFE.contains(TaintColour::PATH_NORMALISED));
        assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::IP_ADDRESS));
        assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::HTML_ESCAPED));
        assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::TAINTED));
    }

    #[test]
    fn t102_safe_excludes_crlf_and_list_colours() {
        // Option-injection safety is purely "cannot start with `-`": CRLF_FREE,
        // LIST_CANONICAL, HTML_ESCAPED, URL_ENCODED prove nothing about the
        // leading byte and must stay out of the T102 mask.
        for c in [
            TaintColour::CRLF_FREE,
            TaintColour::LIST_CANONICAL,
            TaintColour::HTML_ESCAPED,
            TaintColour::URL_ENCODED,
            TaintColour::REGEX_LITERAL,
            TaintColour::SHELL_ATOM,
        ] {
            assert!(
                !TaintColour::T102_SAFE.contains(c),
                "{c:?} must not be T102_SAFE"
            );
        }
    }

    #[test]
    fn all_mask_contains_every_named_colour() {
        // `ALL` is the union the lattice uses as the "definitely clean" mask;
        // every declared bit must be a member.
        for c in [
            TaintColour::TAINTED,
            TaintColour::PATH_PREFIXED,
            TaintColour::NON_DASH_PREFIXED,
            TaintColour::CRLF_FREE,
            TaintColour::SHELL_ATOM,
            TaintColour::LIST_CANONICAL,
            TaintColour::REGEX_LITERAL,
            TaintColour::PATH_NORMALISED,
            TaintColour::PATH_BOUNDED,
            TaintColour::HEADER_TOKEN_SAFE,
            TaintColour::HTML_ESCAPED,
            TaintColour::URL_ENCODED,
            TaintColour::IP_ADDRESS,
            TaintColour::PORT,
            TaintColour::FQDN,
            TaintColour::PATH_JOINED,
            TaintColour::CHANNEL,
        ] {
            assert!(TaintColour::ALL.contains(c), "ALL missing {c:?}");
        }
    }
}

// ===========================================================================
// Lattice join — `sanitised()` / `with()` and join edges not pinned by
// taint.rs (PATH_BOUNDED, HEADER_TOKEN_SAFE survival, the
// REDIRECT_SAFE/CRLF pair, sanitised-then-rejoin).
// ===========================================================================
mod lattice_extra {
    use super::*;

    fn t(c: TaintColour) -> TaintLattice {
        TaintLattice {
            colours: TaintColour::TAINTED | c,
        }
    }

    #[test]
    fn sanitised_clears_taint_keeps_colours() {
        let v = t(TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED).sanitised();
        assert!(!v.is_tainted());
        assert!(v.colours.contains(TaintColour::PATH_NORMALISED));
        assert!(v.colours.contains(TaintColour::PATH_BOUNDED));
    }

    #[test]
    fn sanitised_value_is_join_identity() {
        // After a sanitiser drops TAINTED, the value is untainted, so joining it
        // with a tainted operand must leave the tainted side's colours intact
        // (clean is the identity).
        let clean_again = t(TaintColour::HTML_ESCAPED).sanitised();
        let other = t(TaintColour::CRLF_FREE);
        assert_eq!(clean_again.join(other).colours, other.colours);
    }

    #[test]
    fn path_bounded_survives_self_join() {
        let v = t(TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED);
        let r = v.join(v);
        assert!(r.colours.contains(TaintColour::PATH_BOUNDED));
        assert!(r.colours.contains(TaintColour::PATH_NORMALISED));
    }

    #[test]
    fn header_token_safe_lost_against_generic() {
        let r = t(TaintColour::HEADER_TOKEN_SAFE).join(TaintLattice::tainted());
        assert!(r.is_tainted());
        assert!(!r.colours.contains(TaintColour::HEADER_TOKEN_SAFE));
    }

    #[test]
    fn redirect_pair_intersection_keeps_shared_only() {
        // PATH_PREFIXED on one edge, PATH_NORMALISED on the other → neither
        // survives the must-have intersection, so REDIRECT_SAFE is empty.
        let r = t(TaintColour::PATH_PREFIXED).join(t(TaintColour::PATH_NORMALISED));
        assert!(!r.colours.intersects(TaintColour::REDIRECT_SAFE));
    }

    #[test]
    fn with_is_monotone_add() {
        let base = TaintLattice::tainted();
        let v = base.with(TaintColour::FQDN).with(TaintColour::CRLF_FREE);
        assert!(v.colours.contains(TaintColour::FQDN));
        assert!(v.colours.contains(TaintColour::CRLF_FREE));
        assert!(v.is_tainted());
    }
}

// ===========================================================================
// Source breadth — every taint-source namespace and the colour it stamps.
// Asserted at an `eval` sink (T100 fires regardless of colour) so the test
// proves "this command is a source"; the colour itself is checked via its
// suppression effect in the colour modules below.
// f5-dialect for every iRules getter; `read`/`gets`/`exec`/`socket` are core.
// ===========================================================================
mod source_breadth {
    use super::*;

    /// `<src>` taints `x`, which then reaches `eval` → T100 on `x`.
    fn source_taints(src_cmd: &str) -> bool {
        let snippet = format!("set x [{src_cmd}]\neval $x");
        has(&snippet, D, "T100", "x")
    }

    #[test]
    fn http_query_is_source() {
        // f5-dialect.
        assert!(source_taints("HTTP::query"));
    }

    #[test]
    fn http_method_is_source() {
        // f5-dialect.
        assert!(source_taints("HTTP::method"));
    }

    #[test]
    fn http_cookie_value_is_source() {
        // f5-dialect.
        assert!(source_taints("HTTP::cookie value sid"));
    }

    #[test]
    fn http_username_is_source() {
        // f5-dialect.
        assert!(source_taints("HTTP::username"));
    }

    #[test]
    fn http_password_is_source() {
        // f5-dialect.
        assert!(source_taints("HTTP::password"));
    }

    #[test]
    fn ip_remote_addr_is_source() {
        // f5-dialect.
        assert!(source_taints("IP::remote_addr"));
    }

    #[test]
    fn tcp_remote_port_is_source() {
        // f5-dialect.
        assert!(source_taints("TCP::remote_port"));
    }

    #[test]
    fn udp_payload_is_source() {
        // f5-dialect.
        assert!(source_taints("UDP::payload"));
    }

    #[test]
    fn ssl_payload_is_source() {
        // f5-dialect.
        assert!(source_taints("SSL::payload"));
    }

    #[test]
    fn ssl_cert_is_source() {
        // f5-dialect.
        assert!(source_taints("SSL::cert 0"));
    }

    #[test]
    fn ssl_sni_is_source() {
        // f5-dialect: SSL::sni carries FQDN.
        assert!(source_taints("SSL::sni"));
    }

    #[test]
    fn uri_host_is_source() {
        // f5-dialect: URI:: library getters are bare-TAINTED sources.
        assert!(source_taints("URI::host http://x/"));
    }

    #[test]
    fn uri_query_is_source() {
        // f5-dialect.
        assert!(source_taints("URI::query http://x/?a=1"));
    }

    #[test]
    fn sip_uri_is_source() {
        // f5-dialect.
        assert!(source_taints("SIP::uri"));
    }

    #[test]
    fn encoding_convertfrom_is_source() {
        // tclsh: `encoding convertfrom utf-8 $bytes` decodes attacker bytes to a
        // string — a content value. Core Tcl, subcommand-shaped source.
        assert!(source_taints("encoding convertfrom utf-8 $bytes"));
    }

    #[test]
    fn chan_read_subcommand_is_source() {
        // tclsh: `chan read $fd` is the modern spelling of `read $fd`.
        assert!(source_taints("chan read $fd"));
    }

    #[test]
    fn gets_is_a_trait_source_regardless_of_arity() {
        // tclsh: `gets $fd line` (WITH varName) returns the byte COUNT (proven:
        // `gets $fd line` → `2`), writing the line into `$line`; the getter form
        // `gets $fd` returns the line itself. The analyser classifies `gets` by
        // its `TAINT_SOURCE` *command trait*, not by arity, so it conservatively
        // taints the result of BOTH forms — over-tainting the count form is the
        // safe (no-false-negative) direction. This pins that conservative,
        // arity-agnostic source classification.
        assert!(has("set n [gets $fd line]\neval $n", D, "T100", "n"));
    }
}

// ===========================================================================
// T100 code-execution sinks — the full EVALUATES_CODE / TAINT_SINK family, and
// the AssignValue-embedded-command-substitution path (`set _ [eval $x]`) that
// classifies a sink inside an assignment RHS.
//
// tclsh: eval/uplevel/subst run their argument; unbraced expr re-parses it.
// ===========================================================================
mod t100_sink_family {
    use super::*;

    #[test]
    fn eval_embedded_in_assignment_rhs_fires() {
        // `set out [eval $x]` — the sink (`eval`) is a command substitution on
        // the RHS of an assignment, not a bare Call. The AssignValue branch of
        // `emit_statement_warnings` must still classify it.
        let ws = of_code("set x [read $fd]\nset out [eval $x]", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].sink_command, "eval");
    }

    #[test]
    fn subst_embedded_in_assignment_rhs_fires() {
        // tclsh: `subst {val=$x}` evaluates `$x` (and `[cmd]`); a tainted operand
        // is a code-execution vector even when subst's result is captured.
        let ws = of_code("set x [read $fd]\nset out [subst $x]", D, "T100");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].sink_command, "subst");
    }

    #[test]
    fn expr_braced_operand_message_mentions_coercion() {
        // A braced `expr` operand is the numeric-coercion T100 (not code-exec):
        // the message names the coercion risk, distinguishing it from eval/exec.
        let ws = of_code("set data [read $fd]\nset v [expr {$data * 2}]", D, "T100");
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("expr"));
        assert!(ws[0].message.to_lowercase().contains("coercion"));
    }

    #[test]
    fn two_distinct_tainted_vars_in_eval_each_warn() {
        // Dedup is per-variable, not per-statement: `eval "$a $b"` flags both.
        let ws = of_code(
            "set a [read $f1]\nset b [read $f2]\neval \"$a $b\"",
            D,
            "T100",
        );
        let mut vars: Vec<String> = ws.iter().map(|w| w.variable.clone()).collect();
        vars.sort();
        vars.dedup();
        assert_eq!(vars, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn same_var_twice_in_eval_dedups_to_one() {
        let ws = of_code("set a [read $f1]\neval \"$a and $a\"", D, "T100");
        assert_eq!(ws.iter().filter(|w| w.variable == "a").count(), 1);
    }
}

// ===========================================================================
// T100 SHELL_ATOM suppression at `exec` — the registry `taint_sink_safe_colour`
// path (exec ← SHELL_ATOM). An IP/port/FQDN atom cannot word-split into a new
// exec argument, so it clears T100 at exec; generic taint does not.
// f5-dialect sources.
// ===========================================================================
mod exec_shell_atom {
    use super::*;

    #[test]
    fn ip_atom_clears_exec_t100() {
        // f5-dialect: IP::client_addr augments to SHELL_ATOM → exec-safe.
        assert!(of_code("set a [IP::client_addr]\nexec ping $a", D, "T100").is_empty());
    }

    #[test]
    fn port_atom_clears_exec_t100() {
        // f5-dialect: TCP::client_port → PORT → SHELL_ATOM.
        assert!(of_code("set p [TCP::client_port]\nexec nc host $p", D, "T100").is_empty());
    }

    #[test]
    fn fqdn_atom_clears_exec_t100() {
        // f5-dialect: SSL::sni → FQDN → SHELL_ATOM.
        assert!(of_code("set h [SSL::sni]\nexec host $h", D, "T100").is_empty());
    }

    #[test]
    fn generic_taint_still_fires_exec_t100() {
        // A bare `read` value has no SHELL_ATOM → exec T100 fires.
        let ws = of_code("set x [read $fd]\nexec sh -c $x", D, "T100");
        assert!(ws.iter().any(|w| w.variable == "x"));
    }

    #[test]
    fn exec_also_emits_t102_for_generic_taint() {
        // `exec` declares a `--` terminator, so a leading-`-`-capable tainted
        // value is also option injection: generic `read` → both T100 and T102.
        let cs = codes("set x [read $fd]\nexec $x", D);
        assert!(cs.contains(&"T100".to_string()));
        assert!(cs.contains(&"T102".to_string()));
    }
}

// ===========================================================================
// T101 output-sink position filter — `puts ?-nonewline? ?channelId? string`.
// Only the trailing content word is the sink; a tainted channel handle is not
// injectable. Exercises `sink_var_position_safe` for the `puts` case beyond the
// single taint.rs example (stdout/stderr channel literals, both tainted).
// tclsh: `puts $chan $str` writes $str to channel $chan.
// ===========================================================================
mod puts_position_filter {
    use super::*;

    #[test]
    fn puts_two_arg_channel_then_content_flags_content() {
        // `puts $chan $msg` — slot 0 ($chan) is the destination, slot 1 ($msg)
        // is content. Only $msg trips T101.
        let ws = of_code(
            "set ch [read $f1]\nset msg [read $f2]\nputs $ch $msg",
            D,
            "T101",
        );
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "msg");
    }

    #[test]
    fn puts_literal_channel_tainted_content_flags() {
        let ws = of_code("set msg [read $fd]\nputs stderr $msg", D, "T101");
        assert!(ws.iter().any(|w| w.variable == "msg"));
    }

    #[test]
    fn puts_nonewline_literal_channel_tainted_content_flags() {
        let ws = of_code("set msg [read $fd]\nputs -nonewline stdout $msg", D, "T101");
        assert!(ws.iter().any(|w| w.variable == "msg"));
    }

    #[test]
    fn puts_single_arg_tainted_is_content() {
        // One-arg `puts $x` — $x is the content (default channel stdout).
        let ws = of_code("set x [read $fd]\nputs $x", D, "T101");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "x");
    }
}

// ===========================================================================
// T102 option-injection scan region — `-option value` consumption, the `--`
// terminator inside the arg list, and `{*}`-expansion leading words. Exercises
// `option_scan_region` / `arg_can_be_option` branches beyond taint.rs.
// tclsh: `regexp -- -foo bar` → 0 (-- ends switch scan); `regexp -start N pat …`
// consumes N as the -start value.
// ===========================================================================
mod t102_scan_region {
    use super::*;

    #[test]
    fn value_option_consumes_its_argument() {
        // tclsh: `regexp -start 0 $x subject` — `-start` consumes `0`, so the
        // tainted `$x` lands in the pattern slot (in-region) → T102.
        let ws = of_code("set x [read $fd]\nregexp -start 0 $x subject", D, "T102");
        assert!(ws.iter().any(|w| w.variable == "x"));
    }

    #[test]
    fn terminator_in_arg_list_ends_scan() {
        // `regexp -nocase -- $x subject` — `--` ends switch scanning, so $x is a
        // definite positional (pattern), not an option. No T102.
        assert!(of_code("set x [read $fd]\nregexp -nocase -- $x subject", D, "T102").is_empty());
    }

    #[test]
    fn brace_star_expansion_is_option_capable() {
        // `{*}$x` could expand to a leading `-switch`, so it is in the option
        // scan region → T102.
        let ws = of_code("set x [read $fd]\nregexp {*}$x subject", D, "T102");
        assert!(!ws.is_empty());
    }

    #[test]
    fn literal_positional_before_tainted_subject_ends_scan() {
        // `regexp {fixed} $subject` — the literal `{fixed}` pattern is a definite
        // positional that ends scanning, so the later tainted subject is safe.
        assert!(of_code("set s [read $fd]\nregexp {fixed} $s", D, "T102").is_empty());
    }

    #[test]
    fn exec_option_flag_then_tainted_command_warns() {
        // `exec` declares a `--` terminator with no-value option flags
        // (`-ignorestderr`, `-keepnewline`). A tainted word after such a flag is
        // still in the option-scan region → T102.
        let ws = of_code("set x [read $fd]\nexec -ignorestderr $x", D, "T102");
        assert!(ws.iter().any(|w| w.variable == "x"));
        assert!(ws[0].sink_command.contains("exec"));
    }

    #[test]
    fn exec_with_terminator_clean_for_t102() {
        // `exec -- $x` ends switch scanning, so `$x` cannot be a switch → no
        // T102 (the T100 code-exec sink is a separate concern, not asserted here).
        assert!(of_code("set x [read $fd]\nexec -- $x", D, "T102").is_empty());
    }
}

// ===========================================================================
// T103 regexp-pattern injection — `regexp` AND `regsub` (both PatternType::Regex),
// the `-start`-consumes-value pattern index, and the REGEX_LITERAL suppression
// via the four quote spellings. taint.rs covers `regexp`; this adds
// `regsub`, the pattern-index skip, and the quote-family suppressors.
//
// tclsh: `regsub -all a banana X` → `bXnXnX` (regsub returns the substituted
// string, so a tainted pattern is a live regex-injection vector).
// ===========================================================================
mod t103_regsub_and_quotes {
    use super::*;

    #[test]
    fn regsub_tainted_pattern_warns() {
        // tclsh: regsub's first positional is the regex pattern; tainted → T103.
        let ws = of_code("set x [read $fd]\nregsub $x subject {}", D, "T103");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "x");
        assert!(ws[0].sink_command.contains("regsub"));
    }

    #[test]
    fn regsub_all_flag_then_tainted_pattern_warns() {
        // `-all` is a no-value switch; the pattern is the next positional.
        let ws = of_code("set x [read $fd]\nregsub -all $x subject repl", D, "T103");
        assert!(ws.iter().any(|w| w.variable == "x"));
    }

    #[test]
    fn regsub_tainted_replacement_is_not_t103() {
        // The replacement (slot after pattern) is not a pattern position → no
        // T103 on a tainted replacement when the pattern is literal.
        assert!(of_code("set r [read $fd]\nregsub -all {x} subject $r", D, "T103").is_empty());
    }

    #[test]
    fn regexp_quote_suppresses_t103() {
        // `regexp::quote` stamps REGEX_LITERAL → the pattern is trusted, no T103.
        assert!(
            of_code(
                "set x [read $fd]\nset q [regexp::quote $x]\nregexp $q subject",
                D,
                "T103",
            )
            .is_empty()
        );
    }

    #[test]
    fn regex_colon_quote_suppresses_t103() {
        assert!(
            of_code(
                "set x [read $fd]\nset q [regex::quote $x]\nregexp $q subject",
                D,
                "T103",
            )
            .is_empty()
        );
    }

    #[test]
    fn re_quote_suppresses_t103() {
        assert!(
            of_code(
                "set x [read $fd]\nset q [re_quote $x]\nregsub -all $q subject {}",
                D,
                "T103",
            )
            .is_empty()
        );
    }

    #[test]
    fn regexp_start_consumes_value_pattern_after_warns() {
        // tclsh: `regexp -start 2 a xaxa` → `1`; `-start` consumes its value, so
        // the tainted pattern after it is still the pattern slot → T103.
        let ws = of_code("set x [read $fd]\nregexp -start 2 $x xaxa", D, "T103");
        assert!(ws.iter().any(|w| w.variable == "x"));
    }
}

// ===========================================================================
// T104 SSRF / network-address sinks — `socket` (slots 0,1), `http::geturl`
// (slot 0), and iRules `connect` (empty positions ⇒ any tainted arg). Exercises
// the `taint_network_sink_args` position filter and the IP/PORT/FQDN clearance.
// `socket` is core tclsh; `connect` is f5-dialect.
// ===========================================================================
mod t104_network_sinks {
    use super::*;

    #[test]
    fn socket_host_slot_generic_taint_warns() {
        let ws = of_code("set h [read $fd]\nsocket $h 80", D, "T104");
        assert!(ws.iter().any(|w| w.variable == "h"));
    }

    #[test]
    fn socket_port_slot_generic_taint_warns() {
        // Slot 1 is also a network-address slot per `taint_network_sink_args=[0,1]`.
        let ws = of_code("set p [read $fd]\nsocket localhost $p", D, "T104");
        assert!(ws.iter().any(|w| w.variable == "p"));
    }

    #[test]
    fn socket_validated_ip_host_clears_but_generic_port_warns() {
        // f5-dialect host: IP::client_addr (IP_ADDRESS) clears T104 in slot 0;
        // a generic `read` port in slot 1 still warns.
        let ws = of_code(
            "set a [IP::client_addr]\nset p [read $fd]\nsocket $a $p",
            D,
            "T104",
        );
        assert!(ws.iter().any(|w| w.variable == "p"));
        assert!(!ws.iter().any(|w| w.variable == "a"));
    }

    #[test]
    fn socket_validated_port_clears_slot_one() {
        // f5-dialect: TCP::client_port (PORT) clears T104 in the port slot; the
        // generic host still warns.
        let ws = of_code(
            "set p [TCP::client_port]\nset h [read $fd]\nsocket $h $p",
            D,
            "T104",
        );
        assert!(ws.iter().any(|w| w.variable == "h"));
        assert!(!ws.iter().any(|w| w.variable == "p"));
    }

    #[test]
    fn socket_option_value_not_in_positional_slot() {
        // `socket -myaddr 1.2.3.4 $h 80` — the `-myaddr` value is consumed, so
        // the positional slot 0 is `$h`. Only `$h` warns (the literal `1.2.3.4`
        // option value is not a tainted var anyway, but this pins the
        // option-skipping in `positional_arg_strings`).
        let ws = of_code("set h [read $fd]\nsocket -myaddr 1.2.3.4 $h 80", D, "T104");
        assert!(ws.iter().any(|w| w.variable == "h"));
    }

    #[test]
    fn http_geturl_url_slot_warns() {
        // tcllib `http::geturl` — slot 0 (the URL) is the SSRF vector.
        let ws = of_code("set u [read $fd]\nhttp::geturl $u", D, "T104");
        assert!(ws.iter().any(|w| w.variable == "u"));
    }

    #[test]
    fn http_geturl_fqdn_clears() {
        // f5-dialect source: an SSL::sni FQDN clears T104.
        assert!(of_code("set s [SSL::sni]\nhttp::geturl $s", D, "T104").is_empty());
    }

    #[test]
    fn irules_connect_any_arg_warns() {
        // f5-dialect: `connect` declares `taint_network_sink_args=()` (empty
        // positions ⇒ no slot filter), so any tainted argument is SSRF.
        let ws = of_code("set h [HTTP::header Host]\nconnect $h", IR, "T104");
        assert!(ws.iter().any(|w| w.variable == "h"));
        assert!(ws[0].sink_command.contains("connect"));
    }
}

// ===========================================================================
// T105 cross-interpreter eval — `interp eval` AND `interp invokehidden`, plus
// the literal-`[list <known-cmd>]` head suppression vs the propagated-list case.
// tclsh: `interp eval $i {expr 6*7}` → 42 (runs in the child).
// ===========================================================================
mod t105_cross_interp {
    use super::*;

    #[test]
    fn interp_eval_tainted_script_warns() {
        let ws = of_code("set x [read $fd]\ninterp eval child $x", D, "T105");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "x");
        assert_eq!(ws[0].sink_command, "interp eval");
    }

    #[test]
    fn interp_invokehidden_tainted_warns() {
        let ws = of_code("set x [read $fd]\ninterp invokehidden child $x", D, "T105");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].sink_command, "interp invokehidden");
    }

    #[test]
    fn interp_eval_literal_list_known_cmd_head_suppressed() {
        // `interp eval child [list puts $x]` — the constructed list's command
        // word is the literal known command `puts`, so the tainted `$x` is a
        // quoted argument, not the command word. No T105.
        assert!(
            of_code(
                "set x [read $fd]\ninterp eval child [list puts $x]",
                D,
                "T105"
            )
            .is_empty()
        );
    }

    #[test]
    fn interp_eval_propagated_plain_var_warns() {
        // `set l [list $x]; interp eval child $l` — the `[list]` head suppression
        // requires a *literal* `[list <known-cmd>]` at the call site, invisible
        // through a propagated variable. `$l` is plain tainted (the `list`
        // command stamps no LIST_CANONICAL colour) → T105 fires.
        let ws = of_code(
            "set x [read $fd]\nset l [list $x]\ninterp eval child $l",
            D,
            "T105",
        );
        assert!(ws.iter().any(|w| w.variable == "l"));
    }

    #[test]
    fn interp_eval_subcommand_only_other_subcommands_clean() {
        // `interp share`/`interp delete` are not in `taint_interp_eval_subcommands`
        // → no T105 even with a tainted argument.
        assert!(of_code("set x [read $fd]\ninterp share {} $x child", D, "T105").is_empty());
    }
}

// ===========================================================================
// T106 double-encode — every encoder family (URL via URI::encode /
// URI::encode_component / URI::escape, HTML via HTML::encode / html_encode /
// html_escape, regex via the four quote spellings) re-applied to its own
// already-stamped colour. taint.rs covers URI::encode; this adds the
// sibling spellings and the message label.
// f5-dialect for URI::/HTML::; regex quotes are core/tcllib.
// ===========================================================================
mod t106_double_encode_breadth {
    use super::*;

    #[test]
    fn uri_encode_component_double_warns() {
        // f5-dialect.
        let ws = of_code(
            "set x [HTTP::query]\nset a [URI::encode_component $x]\nset b [URI::encode_component $a]",
            IR,
            "T106",
        );
        assert!(ws.iter().any(|w| w.variable == "a"));
    }

    #[test]
    fn uri_escape_double_warns() {
        // f5-dialect.
        let ws = of_code(
            "set x [HTTP::query]\nset a [URI::escape $x]\nset b [URI::escape $a]",
            IR,
            "T106",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn url_then_url_via_mixed_spellings_warns() {
        // URI::encode stamps URL_ENCODED; URI::escape's double-encode colour is
        // also URL_ENCODED → re-encoding across spellings is still T106.
        let ws = of_code(
            "set x [HTTP::query]\nset a [URI::encode $x]\nset b [URI::escape $a]",
            IR,
            "T106",
        );
        assert!(ws.iter().any(|w| w.variable == "a"));
    }

    #[test]
    fn html_encode_double_warns_with_label() {
        // f5-dialect: the T106 message labels the prior encoding "HTML-escaped".
        let ws = of_code(
            "set x [HTTP::query]\nset a [HTML::encode $x]\nset b [HTML::encode $a]",
            IR,
            "T106",
        );
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("HTML-escaped"));
    }

    #[test]
    fn html_escape_alias_double_warns() {
        // f5-dialect: `html_escape` alias carries the same HTML_ESCAPED colour.
        let ws = of_code(
            "set x [HTTP::query]\nset a [html_escape $x]\nset b [html_escape $a]",
            IR,
            "T106",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn regexp_quote_double_warns_with_label() {
        // Core/tcllib: re-quoting a regex-escaped value double-escapes → T106,
        // labelled "regex-escaped".
        let ws = of_code(
            "set x [read $fd]\nset a [regexp::quote $x]\nset b [regexp::quote $a]",
            D,
            "T106",
        );
        assert!(!ws.is_empty());
        assert!(ws[0].message.contains("regex-escaped"));
    }

    #[test]
    fn single_encode_is_not_double() {
        // f5-dialect: one pass through the encoder is fine — T106 needs the
        // colour already present on the input.
        assert!(of_code("set x [HTTP::query]\nset a [URI::encode $x]", IR, "T106",).is_empty());
    }

    #[test]
    fn url_encode_then_html_encode_is_not_double() {
        // Different colours: URL_ENCODED input into an HTML encoder is a
        // different transform, not a re-encode → no T106.
        assert!(
            of_code(
                "set x [HTTP::query]\nset a [URI::encode $x]\nset b [HTML::encode $a]",
                IR,
                "T106",
            )
            .is_empty()
        );
    }
}

// ===========================================================================
// IRULE3001 (HTTP::respond body) — HTML_ESCAPED suppression, and the
// AssignValue-embedded `[HTTP::respond …]` path. f5-dialect, IR-gated.
// ===========================================================================
mod irule3001_depth {
    use super::*;

    #[test]
    fn html_escaped_body_suppresses() {
        // f5-dialect: HTML::encode stamps HTML_ESCAPED → IRULE3001 cleared.
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
    fn url_encoded_body_does_not_suppress() {
        // URL_ENCODED is NOT an XSS mitigation for an HTML body — only
        // HTML_ESCAPED clears IRULE3001. URI::encode'd content still warns.
        let ws = of_code(
            "set raw [HTTP::query]\nset enc [URI::encode $raw]\nHTTP::respond 200 content $enc",
            IR,
            "IRULE3001",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn generic_tainted_body_warns_with_xss_message() {
        // f5-dialect.
        let ws = of_code(
            "set raw [HTTP::payload]\nHTTP::respond 200 content $raw",
            IR,
            "IRULE3001",
        );
        assert!(!ws.is_empty());
        assert!(
            ws[0].message.to_lowercase().contains("xss")
                || ws[0].message.to_lowercase().contains("content injection")
        );
    }
}

// ===========================================================================
// IRULE3002 (HTTP::header / HTTP::cookie insert|replace value) — the CRLF_SAFE
// suppression family (CRLF_FREE / IP / PORT / FQDN), the `replace` subcommand,
// and that `remove`/`at` are not value sinks. f5-dialect, IR-gated.
// ===========================================================================
mod irule3002_depth {
    use super::*;

    #[test]
    fn header_replace_tainted_value_warns() {
        // f5-dialect: `replace` is a value sink just like `insert`.
        let ws = of_code(
            "set v [HTTP::header Host]\nHTTP::header replace X-Fwd $v",
            IR,
            "IRULE3002",
        );
        assert!(!ws.is_empty());
        assert!(ws[0].sink_command.to_lowercase().contains("header"));
    }

    #[test]
    fn ip_value_clears_header_via_crlf_free() {
        // f5-dialect: an IP address is CRLF-free → IRULE3002 cleared in the
        // value position.
        assert!(
            of_code(
                "set a [IP::client_addr]\nHTTP::header insert X-Real-IP $a",
                IR,
                "IRULE3002",
            )
            .is_empty()
        );
    }

    #[test]
    fn port_value_clears_header() {
        // f5-dialect.
        assert!(
            of_code(
                "set p [TCP::client_port]\nHTTP::header insert X-Port $p",
                IR,
                "IRULE3002",
            )
            .is_empty()
        );
    }

    #[test]
    fn fqdn_value_clears_header() {
        // f5-dialect.
        assert!(
            of_code(
                "set s [SSL::sni]\nHTTP::header insert X-SNI $s",
                IR,
                "IRULE3002",
            )
            .is_empty()
        );
    }

    #[test]
    fn html_encode_value_clears_header_via_crlf_free_component() {
        // The HTML_ESCAPED *colour alone* is not in CRLF_SAFE, but the
        // `HTML::encode` *command* stamps `HTML_ESCAPED | CRLF_FREE` (its
        // `taint_transform`). The CRLF_FREE component IS in CRLF_SAFE → the
        // encoded value clears IRULE3002 in the header value position. This pins
        // the command-transform (not bare-colour) suppression path.
        assert!(
            of_code(
                "set raw [HTTP::header Host]\nset e [HTML::encode $raw]\nHTTP::header insert X-H $e",
                IR,
                "IRULE3002",
            )
            .is_empty()
        );
    }

    #[test]
    fn bare_html_escaped_colour_is_not_crlf_safe() {
        // Lattice-level: the HTML_ESCAPED colour on its own does NOT prove CRLF
        // safety (it rewrites `<`/`>`/`&` but can still carry raw CR/LF), so it is
        // excluded from CRLF_SAFE — the reason the suppression above rides on the
        // command's CRLF_FREE component, not on HTML_ESCAPED.
        assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::HTML_ESCAPED));
        assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::URL_ENCODED));
    }

    #[test]
    fn cookie_replace_tainted_value_warns() {
        // f5-dialect.
        let ws = of_code(
            "set v [HTTP::cookie value sid]\nHTTP::cookie replace sid value $v",
            IR,
            "IRULE3002",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn header_at_subcommand_not_a_sink() {
        // `HTTP::header at` is a getter-ish subcommand, not insert/replace → no
        // IRULE3002 even with a tainted argument.
        assert!(
            of_code(
                "set v [HTTP::header Host]\nHTTP::header at $v",
                IR,
                "IRULE3002",
            )
            .is_empty()
        );
    }
}

// ===========================================================================
// IRULE3003 (log) — the CRLF_SAFE suppression family and the URI::/HTML::encode
// CRLF_FREE augmentation, exercised through `log` value positions beyond
// taint.rs's IP/PORT/SNI/URI/HTML set. f5-dialect, IR-gated.
// ===========================================================================
mod irule3003_depth {
    use super::*;

    #[test]
    fn http_uri_path_colour_does_not_clear_log() {
        // PATH_PREFIXED is a T102 mitigation, NOT a CRLF mitigation: an HTTP::uri
        // (which carries PATH_PREFIXED, not CRLF_FREE) still trips IRULE3003.
        let ws = of_code("set u [HTTP::uri]\nlog local0. $u", IR, "IRULE3003");
        assert!(!ws.is_empty());
    }

    #[test]
    fn uri_escape_clears_log_via_crlf_free() {
        // f5-dialect: URI::escape augments CRLF_FREE → IRULE3003 cleared.
        assert!(
            of_code(
                "set raw [HTTP::query]\nset e [URI::escape $raw]\nlog local0. $e",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }

    #[test]
    fn html_escape_clears_log_via_crlf_free() {
        // f5-dialect: html_escape's transform includes CRLF_FREE.
        assert!(
            of_code(
                "set raw [HTTP::query]\nset e [html_escape $raw]\nlog local0. $e",
                IR,
                "IRULE3003",
            )
            .is_empty()
        );
    }

    #[test]
    fn log_message_mentions_log_injection() {
        // f5-dialect.
        let ws = of_code(
            "set x [HTTP::header User-Agent]\nlog local0. $x",
            IR,
            "IRULE3003",
        );
        assert!(!ws.is_empty());
        assert!(
            ws[0].message.to_lowercase().contains("log injection")
                || ws[0].message.to_lowercase().contains("log forging")
        );
    }
}

// ===========================================================================
// IRULE3004 (HTTP::redirect open-redirect) — the REDIRECT_SAFE family
// (PATH_PREFIXED via HTTP::uri, PATH_NORMALISED via file normalize) vs a generic
// tainted target. taint.rs has an irule3004 module; this adds the
// file-normalize same-origin proof and the not-in-tcl86 gate.
// f5-dialect, IR-gated.
// ===========================================================================
mod irule3004_depth {
    use super::*;

    #[test]
    fn path_prefixed_target_suppresses() {
        // f5-dialect: HTTP::uri starts with `/` (PATH_PREFIXED ⊂ REDIRECT_SAFE)
        // → same-origin, no open redirect.
        assert!(of_code("set u [HTTP::uri]\nHTTP::redirect $u", IR, "IRULE3004").is_empty());
    }

    #[test]
    fn file_normalised_target_suppresses() {
        // PATH_NORMALISED ⊂ REDIRECT_SAFE: a `[file normalize]`d redirect target
        // routes back to the current host.
        assert!(
            of_code(
                "set raw [HTTP::header X-Loc]\nset t [file normalize $raw]\nHTTP::redirect $t",
                IR,
                "IRULE3004",
            )
            .is_empty()
        );
    }

    #[test]
    fn generic_header_target_warns_with_open_redirect_message() {
        // f5-dialect: a raw header value (no path colour) → open redirect.
        let ws = of_code(
            "set loc [HTTP::header Location]\nHTTP::redirect $loc",
            IR,
            "IRULE3004",
        );
        assert!(!ws.is_empty());
        assert!(
            ws[0].message.to_lowercase().contains("open redirect")
                || ws[0].message.to_lowercase().contains("redirect")
        );
    }

    #[test]
    fn redirect_not_classified_under_plain_tcl() {
        // Under tcl8.6 the HTTP::redirect sink is not classified → no IRULE3004
        // even though `read` taints `$loc`.
        assert!(of_code("set loc [read $fd]\nHTTP::redirect $loc", D, "IRULE3004",).is_empty());
    }
}

// ===========================================================================
// IRULE3101 setter-constraint — the three argument shapes (literal / pure
// var-ref / dynamic) and the PATH_NORMALISED/PATH_BOUNDED suppressors beyond
// taint.rs's literal+PATH_PREFIXED cases. f5-dialect, IR-gated.
// ===========================================================================
mod irule3101_depth {
    use super::*;

    #[test]
    fn dynamic_interpolation_target_always_warns() {
        // A `${x}`-interpolated value cannot be proven `/`-prefixed at analysis
        // time → IRULE3101 (the "dynamic" case, distinct from a pure var-ref).
        let ws = of_code("set x [read $fd]\nHTTP::uri \"${x}/p\"", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
        assert!(ws[0].sink_command.contains("HTTP::uri"));
    }

    #[test]
    fn command_sub_target_always_warns() {
        // `[…]` command-sub target — also the dynamic case → warn.
        let ws = of_code("HTTP::uri [HTTP::header X-Path]", IR, "IRULE3101");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn file_normalised_var_suppresses() {
        // PATH_NORMALISED is in the IRULE3101 safe-path set → suppressed.
        assert!(
            of_code(
                "set raw [read $fd]\nset p [file normalize $raw]\nHTTP::path $p",
                IR,
                "IRULE3101",
            )
            .is_empty()
        );
    }

    #[test]
    fn http_path_dynamic_target_warns() {
        let ws = of_code(
            "set seg [read $fd]\nHTTP::path \"api/${seg}\"",
            IR,
            "IRULE3101",
        );
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn literal_with_slash_clean_both_setters() {
        assert!(of_code("HTTP::uri /a/b", IR, "IRULE3101").is_empty());
        assert!(of_code("HTTP::path /a/b", IR, "IRULE3101").is_empty());
    }

    #[test]
    fn not_classified_under_plain_tcl() {
        // The setter constraint is dialect-gated: a literal non-slash target
        // under tcl8.6 emits nothing (defense-in-depth gate).
        assert!(of_code("HTTP::uri relativepath", D, "IRULE3101").is_empty());
    }
}

// ===========================================================================
// W313 destructive-file — the registry `destructive` subcommands (delete /
// rename / mkdir), the `-force`/`--` path skip, multi-path source-order
// determinism, the normalised-but-unguarded softened message, and the
// normalise+`string match` guard suppression. taint.rs does not cover W313.
//
// tclsh: `file delete -force /tmp/td` removes a dir (the `-force` switch and the
// path are real `file delete` args); `file join /base ../x` → `/base/../x` (no
// canonicalisation, so PATH_JOINED ≠ PATH_NORMALISED, does not clear W313).
// ===========================================================================
mod w313_destructive_file {
    use super::*;

    #[test]
    fn file_delete_variable_path_warns() {
        let ws = of_code("set p [read $fd]\nfile delete $p", D, "W313");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].variable, "p");
        assert_eq!(ws[0].sink_command, "file delete");
    }

    #[test]
    fn file_mkdir_variable_path_warns() {
        let ws = of_code("set p [read $fd]\nfile mkdir $p", D, "W313");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].sink_command, "file mkdir");
    }

    #[test]
    fn file_rename_variable_path_warns() {
        // tclsh: `file rename` is a destructive (mutating) op.
        let ws = of_code("set p [read $fd]\nfile rename $p /dest", D, "W313");
        assert!(!ws.is_empty());
        assert_eq!(ws[0].sink_command, "file rename");
    }

    #[test]
    fn force_flag_skipped_to_reach_path() {
        // tclsh: `file delete -force $p` — `-force` is a switch; the path arg is
        // still `$p`, so W313 fires on `$p` (not on the flag).
        let ws = of_code("set p [read $fd]\nfile delete -force $p", D, "W313");
        assert!(ws.iter().any(|w| w.variable == "p"));
    }

    #[test]
    fn literal_path_clean() {
        // tclsh: a literal path cannot carry user content → no W313.
        assert!(of_code("file delete /tmp/known", D, "W313").is_empty());
    }

    #[test]
    fn span_covers_only_the_path_argument() {
        // Range precision: the warning anchors at the dynamic path argument
        // (`$p`), not the whole `file delete …` command — mirroring T102's
        // per-argument targeting.
        let src = "set p [read $fd]\nfile delete $p";
        let ws = of_code(src, D, "W313");
        assert!(!ws.is_empty());
        let expected = src.rfind("$p").unwrap();
        assert_eq!(
            (ws[0].span.start() as usize, ws[0].span.end() as usize),
            (expected, expected + 2),
            "W313 must cover exactly the `$p` argument, got {:?}",
            ws[0].span
        );
    }

    #[test]
    fn span_covers_first_offending_path_argument_of_rename() {
        // `file rename $a $b` warns once, anchored at the first offending
        // path argument (`$a`), not the statement.
        let src = "set a [read $f1]\nset b [read $f2]\nfile rename $a $b";
        let ws = of_code(src, D, "W313");
        assert_eq!(ws.len(), 1);
        let expected = src.rfind("$a").unwrap();
        assert_eq!(
            (ws[0].span.start() as usize, ws[0].span.end() as usize),
            (expected, expected + 2),
            "W313 must cover exactly the `$a` argument, got {:?}",
            ws[0].span
        );
    }

    #[test]
    fn span_skips_force_flag_to_the_path_argument() {
        // With `-force` present the anchor is still the *path* argument, not
        // the switch word.
        let src = "set p [read $fd]\nfile delete -force $p";
        let ws = of_code(src, D, "W313");
        assert!(!ws.is_empty());
        let expected = src.rfind("$p").unwrap();
        assert_eq!(
            (ws[0].span.start() as usize, ws[0].span.end() as usize),
            (expected, expected + 2),
            "W313 must anchor past `-force` at `$p`, got {:?}",
            ws[0].span
        );
    }

    #[test]
    fn first_path_variable_is_deterministic() {
        // `file rename $a $b` — one W313 per statement on the FIRST offending
        // path variable in source order (the `arg_var_names_ordered` determinism
        // guard). `$a` precedes `$b`.
        let ws = of_code(
            "set a [read $f1]\nset b [read $f2]\nfile rename $a $b",
            D,
            "W313",
        );
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].variable, "a");
    }

    #[test]
    fn normalised_unguarded_path_softens_message() {
        // `[file normalize]` proves normalised but not bounded; W313 is not
        // suppressed, only softened — the message asks to verify the directory.
        let ws = of_code(
            "set raw [read $fd]\nset p [file normalize $raw]\nfile delete $p",
            D,
            "W313",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("normalised"));
    }

    #[test]
    fn normalised_and_string_match_guarded_suppressed() {
        // PATH_NORMALISED + a `[string match "/safe/*" $p]` branch guard proves
        // the path stays inside the intended directory → W313 suppressed in the
        // guarded block.
        let src = "set raw [read $fd]\nset p [file normalize $raw]\nif {[string match \"/safe/*\" $p]} {\n  file delete $p\n}";
        assert!(of_code(src, D, "W313").is_empty());
    }

    #[test]
    fn file_join_path_is_not_normalised_still_warns() {
        // tclsh: `file join /base ../x` → `/base/../x` (a portable concat that
        // does NOT collapse `..`). PATH_JOINED is not PATH_NORMALISED, so a
        // `[file join]`d tainted path still trips W313.
        let ws = of_code(
            "set raw [read $fd]\nset p [file join /base $raw]\nfile delete $p",
            D,
            "W313",
        );
        assert!(ws.iter().any(|w| w.variable == "p"));
    }

    #[test]
    fn file_stat_non_destructive_subcommand_clean() {
        // `file stat`/`file exists` are non-destructive reads → no W313.
        assert!(of_code("set p [read $fd]\nfile exists $p", D, "W313").is_empty());
    }
}

// ===========================================================================
// Transform-colour propagation through copies and interpolation — PATH_JOINED
// (file join), PATH_NORMALISED (file normalize), URL_ENCODED / HTML_ESCAPED
// survive a plain `set y $x` copy but are cleared by interpolation
// (`interpolation_carve_out`). These do NOT suppress T100 (the value still
// reaches a code-execution sink). f5-dialect for URI::/HTML:: sources.
// ===========================================================================
mod transform_propagation {
    use super::*;

    #[test]
    fn path_joined_does_not_suppress_t100() {
        // `[file join]` makes a path portable, not eval-safe → T100 still fires.
        let ws = of_code(
            "set raw [read $fd]\nset p [file join /base $raw]\neval $p",
            D,
            "T100",
        );
        assert!(!ws.is_empty());
    }

    #[test]
    fn url_encoded_copy_then_double_encode_still_detected() {
        // URL_ENCODED survives a plain copy `set b $a`, so re-encoding `$b`
        // double-encodes → T106 (on `$b`, the copy carrying the colour).
        let ws = of_code(
            "set x [HTTP::query]\nset a [URI::encode $x]\nset b $a\nset c [URI::encode $b]",
            IR,
            "T106",
        );
        assert!(ws.iter().any(|w| w.variable == "b"));
    }

    #[test]
    fn html_escaped_interpolation_clears_double_encode() {
        // Interpolation `"<p>${a}</p>"` invalidates HTML_ESCAPED
        // (`interpolation_carve_out`), so passing the interpolated value through
        // HTML::encode is a FRESH encode, not a double-encode → no T106.
        assert!(
            of_code(
                "set x [HTTP::query]\nset a [HTML::encode $x]\nset b \"<p>${a}</p>\"\nset c [HTML::encode $b]",
                IR,
                "T106",
            )
            .is_empty()
        );
    }

    #[test]
    fn path_normalised_interpolation_appends_clears_colour() {
        // `"${norm}/extra"` re-introduces unvalidated structure → PATH_NORMALISED
        // lost, so a `file delete` of the interpolated value is the un-normalised
        // (harder) W313 message, not the softened one.
        let ws = of_code(
            "set raw [read $fd]\nset norm [file normalize $raw]\nset p \"${norm}/x\"\nfile delete $p",
            D,
            "W313",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("path-traversal"));
    }
}

// ===========================================================================
// Sanitiser breadth — fixed-numeric-return subcommands strip taint so the
// result clears every sink. taint.rs covers `string length` / `llength`;
// this adds `string is`, `string match`, `string compare`, and confirms a
// non-numeric-return string op (`string toupper`) does NOT sanitise.
//
// tclsh: `string toupper "abc-def"` → `ABC-DEF` (a content string of the same
// shape — preserves a leading `-`, so it cannot be a taint sanitiser).
// ===========================================================================
mod sanitiser_breadth {
    use super::*;

    #[test]
    fn string_is_integer_sanitises() {
        // tclsh: `string is integer …` returns a boolean (0/1) — a fixed numeric
        // return that cannot carry content taint.
        assert!(
            codes(
                "set x [read $fd]\nset ok [string is integer $x]\neval $ok",
                D,
            )
            .is_empty()
        );
    }

    #[test]
    fn string_compare_sanitises() {
        // tclsh: `string compare a b` → an int in {-1,0,1}.
        assert!(
            codes(
                "set x [read $fd]\nset c [string compare $x foo]\nexpr $c",
                D,
            )
            .is_empty()
        );
    }

    #[test]
    fn string_match_sanitises() {
        // tclsh: `string match pat str` → boolean.
        assert!(codes("set x [read $fd]\nset m [string match a* $x]\nexpr $m", D,).is_empty());
    }

    #[test]
    fn string_toupper_does_not_sanitise() {
        // tclsh: `string toupper "abc-def"` → `ABC-DEF` (content string, same
        // shape) — NOT a fixed-numeric return, so taint flows through to eval.
        let ws = of_code(
            "set x [read $fd]\nset u [string toupper $x]\neval $u",
            D,
            "T100",
        );
        assert!(ws.iter().any(|w| w.variable == "u"));
    }

    #[test]
    fn sanitiser_inside_command_sub_clears_argument() {
        // A sanitiser used as an embedded `[string length $x]` directly in a
        // sink's argument should clear the taint of that argument word — tclsh
        // proves `puts [string length $x]` outputs a NUMBER (`25` for a 25-char
        // value), never `$x`'s content, so there is no injectable output.
        //
        // The `expr` sink already gets this right (taint.rs proves
        // `expr {[string length $data]}` is silent), but the `puts`/`eval`/`exec`
        // sink loop (`emit_sink_warnings`) iterates the statement's SSA `uses`
        // directly and treats every `$var` syntactically present in the sink
        // argument as flowing in — even one consumed by an embedded sanitiser.
        //
        // FIXED: `emit_sink_warnings` now applies the embedded-sanitiser
        // carve-out (mirroring the expr/word_taint path), so the sanitiser-
        // consumed `$x` no longer false-fires T101.
        assert!(!has(
            "set x [read $fd]\nputs [string length $x]",
            D,
            "T101",
            "x"
        ));
        // Control: a bare tainted value in the same sink still fires — the
        // carve-out is targeted (only vars fully consumed by a sanitiser), not
        // a blanket suppression.
        assert!(has("set x [read $fd]\nputs $x", D, "T101", "x"));
    }
}

// ===========================================================================
// Dialect gating — every iRules sink code is silent under plain tcl8.6 even
// when the data is genuinely tainted, and every iRules SOURCE still taints
// under tcl8.6 (registered globally). Pins the source/sink dialect asymmetry
// that the whole-file premise relies on.
// ===========================================================================
mod dialect_gating {
    use super::*;

    #[test]
    fn all_irule_sinks_silent_under_plain_tcl() {
        // Tainted data into HTTP::respond / HTTP::header / HTTP::redirect / log
        // under tcl8.6 → none of the IRULE codes fire.
        let src = "set d [read $fd]\nHTTP::respond 200 content $d\nHTTP::header insert X $d\nHTTP::redirect $d\nlog local0. $d";
        let cs = codes(src, D);
        assert!(!cs.iter().any(|c| c.starts_with("IRULE")));
    }

    #[test]
    fn irules_source_taints_under_plain_tcl() {
        // HTTP::uri is a global source → T100 at eval even under tcl8.6.
        assert!(has("set u [HTTP::uri]\neval $u", D, "T100", "u"));
    }

    #[test]
    fn irules_sink_fires_under_irules_dialect() {
        // Control: the same HTTP::respond IS classified under f5-irules.
        assert!(
            !of_code(
                "set d [HTTP::payload]\nHTTP::respond 200 content $d",
                IR,
                "IRULE3001",
            )
            .is_empty()
        );
    }

    #[test]
    fn generic_t100_t101_work_in_both_dialects() {
        // eval/puts sinks are dialect-agnostic.
        assert!(has("set x [read $fd]\neval $x", IR, "T100", "x"));
        assert!(has("set x [read $fd]\nputs $x", IR, "T101", "x"));
    }
}

// ===========================================================================
// Interprocedural depth — colour-aware return summaries: a helper that
// passes through its argument carries the argument's taint AND colour to the
// caller; a helper that sanitises returns clean; a helper that introduces a
// source taints regardless of arguments. taint.rs covers the basic
// passthrough/sanitise/source cases; this pins colour transfer and the
// option-injection suppression surviving a passthrough.
// ===========================================================================
mod interproc_depth {
    use super::*;

    #[test]
    fn passthrough_carries_taint_into_eval() {
        // A bare `return $x` passthrough carries the argument's TAINTED bit to the
        // caller, so the returned value is a T100 code-execution risk at `eval`.
        let src = "proc wrap {x} { return $x }\nset raw [read $fd]\nset w [wrap $raw]\neval $w";
        assert!(has(src, D, "T100", "w"));
    }

    #[test]
    fn passthrough_conservatively_loses_source_option_safety_colour() {
        // A direct copy preserves the source's PATH_PREFIXED (so `regexp $w` is
        // T102-clean — taint.rs `path_prefixed_copy_suppresses`), but the
        // interprocedural return summary for a bare `return $x` passthrough does
        // NOT carry the source-derived option-safety colour through the proc
        // boundary: the caller's `$w` is plain TAINTED, so T102 fires. This is
        // the conservative (over-warning, no-false-negative) direction and
        // matches taint.rs `helper_passthrough_generic_taint_fires_t102`;
        // it is pinned here for the *coloured*-source case (HTTP::uri, which is
        // T102-safe when copied directly but not when passed through `wrap`).
        let src =
            "proc wrap {x} { return $x }\nset uri [HTTP::uri]\nset w [wrap $uri]\nregexp $w test";
        assert!(!of_code(src, D, "T102").is_empty());
    }

    #[test]
    fn sanitising_helper_result_via_var_clears_t101() {
        // tclsh: the helper returns `string length` (an int) → clean. Captured
        // into `$n` first (the canonical sanitiser-propagation form), `puts $n`
        // is T101-clean. (The embedded form `puts [len $raw]` hits the
        // emit_sink_warnings false positive documented in w313/sanitiser_breadth
        // — avoided here by binding the result to a variable.)
        let src = "proc len {x} { return [string length $x] }\nset raw [read $fd]\nset n [len $raw]\nputs $n";
        assert!(of_code(src, D, "T101").is_empty());
    }

    #[test]
    fn source_returning_helper_taints_t104() {
        // f5-dialect: a helper returning HTTP::header (generic taint) feeding a
        // socket address → T104 at the caller.
        let src = "proc host {} { return [HTTP::header Host] }\nset h [host]\nsocket $h 80";
        assert!(has(src, D, "T104", "h"));
    }

    #[test]
    fn tainted_arg_into_helper_with_internal_sink_warns() {
        // The sink lives inside the helper; the tainted actual flows in and trips
        // it (the entry-taint seeding path).
        let src = "proc run {cmd} { eval $cmd }\nset raw [read $fd]\nrun $raw";
        let ws = of_code(src, D, "T100");
        assert!(ws.iter().any(|w| w.sink_command == "eval"));
    }
}
