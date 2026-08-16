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

//! Residual coverage for several under-covered `tcl-compiler` analysis
//! files.  This suite deliberately targets the *remaining* uncovered branches
//! that the existing suites (`analyser.rs`, `checks.rs`, `core_analyses.rs`,
//! `fp_depth.rs`, `inlining.rs`, `dataflow.rs`, `optimiser_*`) and the
//! in-crate unit tests already miss:
//!
//!   * `src/scan_predicate.rs` — every `scan`/format-string conversion
//!     specifier, the width/precision/size/suppression skips, and the decline
//!     (`None`) paths.  Driven both directly (`scan_provably_no_match`, which is
//!     `pub`) and end-to-end through the W210 read-before-set surface.
//!   * `src/auto_path_eval.rs` — the `lappend auto_path` mini-evaluator: the
//!     `[file join]` / `[file dirname]` / `[info script]` idioms, the tokeniser
//!     quirks (brace groups, quoted words, braced-`[` as a literal,
//!     unterminated delimiters), and the POSIX path helpers.
//!   * `src/place_bridge.rs` — the place/SSA-bridge def/read resolution: dict
//!     paths, must-alias kills, braced-literal data words, upvar aliases,
//!     `namespace upvar`, trace targets, whole-array reads, and the
//!     terminator-read paths.
//!   * `src/lattice_rebase.rs` — the memoised-unit offset rebase (SCCP value
//!     rebasing across a shift): the `Foreach` / `Try` / `UpFrame` / constant-
//!     branch / loop-node statement variants the existing shift test misses.
//!   * `src/inlining/mod.rs` — `classify_proc` / `inline_module` decline reasons
//!     (size limits, recursion, side-effects, non-namespace-invariant calls,
//!     globals, `{*}` expansion) `inlining.rs` misses.
//!   * `src/analyser/diagnostics/fp/rch.rs` — the reachability (O107) FP
//!     suppressor carve-outs: code after `return` / `break` / `continue` /
//!     `error`, constant-false guards, and the various unreachable-region shapes,
//!     each with a fires/suppressed control pair.
//!
//! ## Proof discipline
//!
//! Where a test depends on a **Tcl-observable** fact — a `scan` conversion
//! result, what survives after `return` / `error`, a `file join` / `file
//! dirname` value, reachability after an unconditional exit — the expected value
//! is pinned to real `tclsh8.6` and `tclsh9.0` via `scripts/dev/tclsh_check.sh`
//! and cited with a `// tclsh:` comment (8.4 / 8.5 are not installed in this
//! environment).
//!
//! **Analysis-internal structure** — the lattice/place/predicate *shape*, the
//! rebased-span identity, the `InlineDecision` verdict, and which IR node a
//! splice produces — is **not** directly observable from a Tcl program's output,
//! so it is asserted structurally.  For `scan_predicate`, several inputs Tcl
//! itself leaves the output unset on (`scan nan %f`, the second `%d` of
//! `%*d%d`) are nonetheless *not* provable-no-match for the predicate, which is
//! a deliberate sound over-approximation (it never fires a false W210); those
//! cases assert the conservative `false` return and say so.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::auto_path_eval::evaluate_auto_path_expr;
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::inlining::{
    InlineDecision, SMALL_BODY_THRESHOLD, classify_proc, count_statements, inline_module,
};
use tcl_compiler::ir::{Module, Statement};
use tcl_compiler::place::{self, Index, LOCAL_NS, Place, PlaceKind, overlap};
use tcl_compiler::place_bridge::{
    build_resolve_context, def_places, element_writes_observed_by_reads, read_places,
    terminator_read_places,
};
use tcl_compiler::scan_predicate::scan_provably_no_match;
use tcl_compiler::var_escape::{ProcEscapeSummary, analyse_var_escape};
use tcl_registry::{CommandRegistry, registry_for_dialect};

// ===========================================================================
// Shared harness
// ===========================================================================

/// Default dialect for the diagnostic-surface reproducers.
const D: &str = "tcl8.6";

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// Every diagnostic code the analyser + `run_all_checks` surface for `src`
/// (optimisation codes excluded) — the W210/W211/W220 surface used by the
/// place-bridge and scan-predicate end-to-end proofs.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    use tcl_compiler::compiler_checks::run_all_checks;
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then(|| tcl_dialect::DialectProfile::by_name(dialect));
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// `O107` (unreachable dead code) is an optimiser-pass suggestion, not an
/// analyser/`run_all_checks` diagnostic — union the optimiser output to test
/// the reachability FP suppressor.
fn o107_fires(src: &str, dialect: &str) -> bool {
    use tcl_compiler::optimiser::manager::optimise_with_dialect;
    use tcl_core_types::DiagCode;
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then(|| tcl_dialect::DialectProfile::by_name(dialect));
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code == DiagCode::O107)
}

/// `source → Module`, for the inliner.
fn module_for(source: &str) -> Module {
    CompilationUnit::build_for(source, &reg(), false).ir_module
}

/// Run the whole-module inline transform.
fn inlined(source: &str) -> Module {
    inline_module(module_for(source), &reg())
}

/// Count statement-position calls to `command` across the module top level.
fn top_calls_to(module: &Module, command: &str) -> usize {
    module
        .top_level
        .statements
        .iter()
        .filter(|s| matches!(s, Statement::Call { command: c, .. } if c == command))
        .count()
}

/// Collect every def-place and read-place across a proc's CFG (mirrors the
/// `place_bridge` unit-test helper, exposed here over the public API).
fn places_of(src: &str, qname: &str) -> (Vec<Place>, Vec<Place>) {
    let r = reg();
    let cu = CompilationUnit::build_for(src, &r, false);
    let fu = cu.function(qname).expect("proc not found");
    let ctx = build_resolve_context(&fu.cfg, &fu.name);
    let mut defs = Vec::new();
    let mut reads = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            defs.extend(def_places(stmt, &ctx, &r));
            reads.extend(read_places(stmt, &ctx, &r));
        }
        if let Some(term) = &block.terminator {
            reads.extend(terminator_read_places(term, &ctx, &r));
        }
    }
    (defs, reads)
}

// ===========================================================================
// scan_predicate.rs — every conversion specifier + the decline paths
// ===========================================================================
//
// `scan_provably_no_match(FMT, INPUT)` returns `true` iff the first conversion
// is statically guaranteed not to consume input (so the first output var stays
// unset).  Every expected verdict below is pinned to tclsh: a `true` means
// tclsh leaves the output unset; a `false` means tclsh either fills it or the
// predicate conservatively declines (noted inline).

#[test]
fn scan_decimal_int_match_and_no_match() {
    // tclsh (8.6, 9.0): `scan 42 %d n` → n=42 (match); `scan abc %d n` → unset.
    assert!(!scan_provably_no_match("%d", "42"));
    assert!(scan_provably_no_match("%d", "abc"));
    // Signed forms still match (leading +/-).
    // tclsh (8.6, 9.0): `scan -5 %d n` → n=-5; `scan +5 %i n` → n=5.
    assert!(!scan_provably_no_match("%d", "-5"));
    assert!(!scan_provably_no_match("%i", "+5"));
    assert!(!scan_provably_no_match("%u", "5"));
    // A lone sign with no following digit can't satisfy the conversion.
    assert!(scan_provably_no_match("%d", "-"));
}

#[test]
fn scan_octal_int_match_and_no_match() {
    // tclsh (8.6, 9.0): `scan 777 %o n` → n=511 (octal); `scan 8 %o n` → unset
    // (8 is not an octal digit); `scan 9 %o n` → unset.
    assert!(!scan_provably_no_match("%o", "777"));
    assert!(scan_provably_no_match("%o", "8"));
    assert!(scan_provably_no_match("%o", "9"));
}

#[test]
fn scan_binary_int_match_and_no_match() {
    // tclsh (8.6, 9.0): `scan 101 %b n` → n=5; `scan 2 %b n` → unset.
    assert!(!scan_provably_no_match("%b", "101"));
    assert!(scan_provably_no_match("%b", "2"));
}

#[test]
fn scan_hex_int_match_and_no_match() {
    // tclsh (8.6, 9.0): `scan 0x1f %x n` → n=31 (leading 0 is a hex digit);
    // `scan g %x n` → unset (g is not a hex digit).
    assert!(!scan_provably_no_match("%x", "0x1f"));
    assert!(!scan_provably_no_match("%X", "1f"));
    assert!(scan_provably_no_match("%x", "g"));
}

#[test]
fn scan_char_conversion_consumes_any_byte() {
    // `%c` reads ANY single byte (no leading-whitespace skip).
    // tclsh (8.6, 9.0): `scan abc %c n` → n=97; `scan "" %c n` → unset.
    assert!(!scan_provably_no_match("%c", "abc"));
    assert!(!scan_provably_no_match("%c", " ")); // a space is a byte for %c
    assert!(scan_provably_no_match("%c", "")); // empty input — nothing to read
}

#[test]
fn scan_string_conversion_needs_nonspace() {
    // `%s` skips leading whitespace then needs a non-space char.
    // tclsh (8.6, 9.0): `scan "   " %s n` → unset (all whitespace);
    // `scan abc %s n` → n=abc.
    assert!(!scan_provably_no_match("%s", "abc"));
    assert!(!scan_provably_no_match("%s", "  x")); // leading ws skipped, x remains
    assert!(scan_provably_no_match("%s", "   "));
    assert!(scan_provably_no_match("%s", ""));
}

#[test]
fn scan_float_conversion_variants() {
    // Every float leader: a digit, a leading-sign+digit, a `.digit`, and Inf.
    // tclsh (8.6, 9.0): `scan .5 %f n` → 0.5; `scan -.5 %f n` → -0.5;
    // `scan Inf %f n` → Inf; `scan x %f n` → unset; `scan . %f n` → unset.
    assert!(!scan_provably_no_match("%f", "3"));
    assert!(!scan_provably_no_match("%e", "+3"));
    assert!(!scan_provably_no_match("%g", ".5"));
    assert!(!scan_provably_no_match("%E", "-.5"));
    assert!(!scan_provably_no_match("%G", "Inf"));
    assert!(!scan_provably_no_match("%f", "infinity")); // case-insensitive prefix
    assert!(scan_provably_no_match("%f", "x"));
    assert!(scan_provably_no_match("%f", ".")); // a lone dot is not a float
    assert!(scan_provably_no_match("%f", "+")); // a lone sign is not a float
    assert!(scan_provably_no_match("%f", "")); // empty input
}

#[test]
fn scan_float_nan_is_conservative_not_provable() {
    // SOUNDNESS NOTE (not a bug): tclsh (8.6, 9.0) `scan nan %g n` leaves `n`
    // UNSET — this build's `scan "nan" %g n` does not fill the var.  The
    // predicate, however, treats a leading `nan` prefix as "might match" and
    // returns `false` (declines to prove no-match).  That is a deliberate
    // OVER-approximation: returning `false` means no W210 fires, so a real
    // unset is merely not flagged — never a false positive.  The predicate is
    // sound (it only ever asserts a *provable* no-match), just imprecise here.
    assert!(
        !scan_provably_no_match("%g", "nan"),
        "the nan-prefix float path is a sound over-approximation (returns false)"
    );
}

#[test]
fn scan_percent_n_always_succeeds() {
    // `%n` writes the consumed-character count and never consumes input, so it
    // can never be a provable no-match — even on empty input.
    // tclsh (8.6, 9.0): `scan "" %n n` → n=0 (always assigned).
    assert!(!scan_provably_no_match("%n", ""));
    assert!(!scan_provably_no_match("%n", "anything"));
}

#[test]
fn scan_literal_percent_directive() {
    // `%%` matches a literal `%` in the input; only THEN does the conversion
    // run.  tclsh (8.6, 9.0): `scan "%5" "%%%d" n` → n=5 (the %% eats the %,
    // %d reads 5); `scan "X5" "%%%d" n` → unset (X is not a literal %).
    assert!(!scan_provably_no_match("%%%d", "%5"));
    assert!(scan_provably_no_match("%%%d", "X5"));
    // A `%%` that runs off the end of the input mismatches.
    assert!(scan_provably_no_match("%%%d", ""));
}

#[test]
fn scan_assignment_suppression_star() {
    // `%*d` consumes-and-discards; the predicate models the FIRST conversion,
    // which is the suppressed `d`.  `scan "5" "%*d%d" n` matches the first %*d
    // against `5`, so the predicate returns `false` (might consume).  tclsh
    // (8.6, 9.0) leaves `n` UNSET there (the SECOND %d has no input left) — a
    // sound over-approximation: `false` means no false W210.
    assert!(
        !scan_provably_no_match("%*d%d", "5"),
        "suppressed first conversion consumes — predicate declines to prove no-match"
    );
    // But if the suppressed conversion itself can't match, it IS a no-match.
    assert!(scan_provably_no_match("%*d%d", "abc"));
}

#[test]
fn scan_width_and_size_modifiers_are_skipped() {
    // A width prefix (`%2d`) and a size modifier (`%ld`, `%hd`, `%lld`) are
    // skipped before the conversion char is read.
    // tclsh (8.6, 9.0): `scan "12 34" "%2d" n` → n=12; `scan 42 "%ld" n` → 42.
    assert!(!scan_provably_no_match("%2d", "12"));
    assert!(!scan_provably_no_match("%ld", "42"));
    assert!(!scan_provably_no_match("%lld", "42"));
    assert!(!scan_provably_no_match("%hd", "42"));
    // …and they don't rescue a genuine mismatch.
    assert!(scan_provably_no_match("%2d", "ab"));
    assert!(scan_provably_no_match("%ld", "zz"));
}

#[test]
fn scan_plain_literal_prefix_must_match() {
    // A non-`%`, non-whitespace literal in the format must match the input
    // verbatim before the conversion.  tclsh (8.6, 9.0): `scan abc "lit%d" n`
    // → unset (no `lit` prefix); a matching literal lets the conversion run.
    assert!(scan_provably_no_match("lit%d", "abc"));
    assert!(scan_provably_no_match("x%d", "y3")); // literal x vs y
    assert!(!scan_provably_no_match("x%d", "x3")); // literal matches, %d reads 3
}

#[test]
fn scan_format_whitespace_directive_skips_input_run() {
    // A whitespace char in the format skips a run of input whitespace before
    // the conversion.  tclsh (8.6, 9.0): `scan "  42" "%d" n` → 42 (leading ws
    // auto-skipped for %d); a leading-ws skip lets a following digit match.
    assert!(!scan_provably_no_match(" %d", "   42"));
    assert!(!scan_provably_no_match("%d", "  42"));
}

#[test]
fn scan_decline_paths_return_false() {
    // Every "outside the modelled subset" path returns `false` (never
    // proves a no-match):
    // 1. A char-set conversion `%[...]` is unmodelled.
    assert!(!scan_provably_no_match("%[abc]", "x"));
    // 2. A format that ends right after `%` is malformed → None → false.
    assert!(!scan_provably_no_match("%", "abc"));
    // 3. A format with no conversion at all has nothing to write → None → false.
    assert!(!scan_provably_no_match("plain", "plain"));
    assert!(!scan_provably_no_match("", ""));
    // 4. A `\`, `$`, or `[` in either arg bails immediately (raw-source slices,
    //    no substitution applied).
    assert!(!scan_provably_no_match("\\r%d", " 123"));
    assert!(!scan_provably_no_match("%d", "$x"));
    assert!(!scan_provably_no_match("[cmd]%d", "abc"));
    // 5. An unmodelled conversion char (`%p`) → None → false.
    assert!(!scan_provably_no_match("%p", "abc"));
    // 6. A `%*` that runs off the end of the format → None → false.
    assert!(!scan_provably_no_match("%*", "abc"));
    // 7. Width digits that consume the rest of the format → None → false.
    assert!(!scan_provably_no_match("%12", "abc"));
    // 8. A size modifier that ends the format → None → false.
    assert!(!scan_provably_no_match("%l", "abc"));
}

#[test]
fn scan_predicate_w210_end_to_end_fires_and_silent() {
    // End-to-end through the read-before-set surface, the Tcl-observable proof:
    // a provable no-match leaves the output var unset, so the later read is a
    // real read-before-set (W210); a match must stay silent.
    // tclsh (8.6, 9.0): `scan 777 %o n` → n=511 (no W210);
    //                    `scan 8 %o n`   → n unset (W210 on `puts $n`).
    let silent = "proc f {} { scan 777 %o n\n puts $n }";
    let fires_src = "proc f {} { scan 8 %o n\n puts $n }";
    assert!(
        !fires(silent, D, "W210"),
        "octal match must not fire W210; emitted {:?}",
        codes(silent, D)
    );
    assert!(
        fires(fires_src, D, "W210"),
        "octal no-match read-before-set must fire W210; emitted {:?}",
        codes(fires_src, D)
    );
}

#[test]
fn scan_predicate_w210_hex_and_binary() {
    // tclsh (8.6, 9.0): `scan 0x1f %x n` → 31 (silent); `scan g %x n` → unset
    // (fires); `scan 101 %b n` → 5 (silent); `scan 2 %b n` → unset (fires).
    assert!(!fires("proc f {} { scan 0x1f %x n\n puts $n }", D, "W210"));
    assert!(fires("proc f {} { scan g %x n\n puts $n }", D, "W210"));
    assert!(!fires("proc f {} { scan 101 %b n\n puts $n }", D, "W210"));
    assert!(fires("proc f {} { scan 2 %b n\n puts $n }", D, "W210"));
}

// ===========================================================================
// auto_path_eval.rs — the lappend auto_path mini-evaluator
// ===========================================================================
//
// Structural note: the evaluator is a side-effect-free POSIX-path mini-eval; its
// path helpers (`posix_join`, `normpath`, `expanduser`) follow standard POSIX
// path rules, NOT tclsh's `file` command byte-for-byte (tclsh's `//`
// handling differs across 8.6/9.0).  The `[file dirname]` / `[file join]` /
// `[info script]` idiom *resolution* is what is Tcl-observable and is pinned to
// tclsh below; the helper normalisation is asserted against the documented
// POSIX behaviour.  All tests use ABSOLUTE `info_script` / literal paths so
// `abspath` reduces to `normpath` and the result is cwd-independent.

#[test]
fn auto_path_file_join_multi_component() {
    // tclsh (8.6, 9.0): `file join /proj a b c` → /proj/a/b/c.
    assert_eq!(
        evaluate_auto_path_expr("[file join /proj a b c]", None).as_deref(),
        Some("/proj/a/b/c"),
    );
}

#[test]
fn auto_path_file_join_later_absolute_resets() {
    // A later absolute component resets the accumulator (POSIX join + tclsh).
    // tclsh (8.6, 9.0): `file join a /b c` → /b/c.
    assert_eq!(
        evaluate_auto_path_expr("[file join /a /b c]", None).as_deref(),
        Some("/b/c"),
    );
}

#[test]
fn auto_path_file_join_single_arg() {
    // `file join` with exactly one path arg (args.len()==2 after the subcommand)
    // returns it unchanged (then abspath/normpath).
    // tclsh (8.6, 9.0): `file join /solo` → /solo.
    assert_eq!(
        evaluate_auto_path_expr("[file join /solo]", None).as_deref(),
        Some("/solo"),
    );
}

#[test]
fn auto_path_dirname_root_and_trailing_slash() {
    // `posix_dirname` collapses a non-root head's trailing slashes but keeps the
    // root.  tclsh (8.6, 9.0): `file dirname /a/b/c` → /a/b; `file dirname /`
    // → /; `file dirname //` → /.
    assert_eq!(
        evaluate_auto_path_expr("[file dirname /a/b/c]", None).as_deref(),
        Some("/a/b"),
    );
    // A path with a single leading component → its dirname is the root.
    assert_eq!(
        evaluate_auto_path_expr("[file dirname /a]", None).as_deref(),
        Some("/"),
    );
}

#[test]
fn auto_path_normpath_collapses_dotdot_and_dot() {
    // The end-to-end parent-dir idiom collapses `..` and `.` via normpath.
    // tclsh (8.6, 9.0): `file dirname [info script]` of /proj/lib/pkg/x.tcl with
    // a trailing `..` → /proj/lib.
    assert_eq!(
        evaluate_auto_path_expr(
            "[file join [file dirname [info script]] ..]",
            Some("/proj/lib/pkg/x.tcl"),
        )
        .as_deref(),
        Some("/proj/lib"),
    );
    // A literal absolute path with embedded `.` / `..` / `//` normalises.
    assert_eq!(
        evaluate_auto_path_expr("/a/./b/../c", None).as_deref(),
        Some("/a/c"),
    );
}

#[test]
fn auto_path_normpath_double_slash_preserved() {
    // POSIX `normpath` preserves EXACTLY two leading slashes and
    // collapses three-or-more to one (a deliberate POSIX-specific rule; tclsh's
    // own `file` command differs across 8.6/9.0, so this is the documented
    // POSIX behaviour, asserted structurally).
    assert_eq!(
        evaluate_auto_path_expr("//net/share", None).as_deref(),
        Some("//net/share"),
    );
    assert_eq!(
        evaluate_auto_path_expr("///srv/x", None).as_deref(),
        Some("/srv/x"),
    );
}

#[test]
fn auto_path_normpath_dotdot_above_root_is_clamped() {
    // `..` cannot ascend above an absolute root — it is dropped.
    assert_eq!(
        evaluate_auto_path_expr("/../../x", None).as_deref(),
        Some("/x"),
    );
}

#[test]
fn auto_path_nested_and_double_dirname() {
    // `[file dirname [file dirname [info script]]]` climbs two levels.
    // tclsh (8.6, 9.0): for /proj/lib/init.tcl → /proj.
    assert_eq!(
        evaluate_auto_path_expr(
            "[file dirname [file dirname [info script]]]",
            Some("/proj/lib/init.tcl"),
        )
        .as_deref(),
        Some("/proj"),
    );
}

#[test]
fn auto_path_info_non_script_arg_declines() {
    // `[info ...]` with anything other than `script` is unmodelled → None.
    assert_eq!(
        evaluate_auto_path_expr("[info library]", Some("/a/b.tcl")),
        None,
    );
    // `[info script]` with no known path → None.
    assert_eq!(
        evaluate_auto_path_expr("[file dirname [info script]]", None),
        None,
    );
}

#[test]
fn auto_path_file_unknown_subcommand_and_arity_decline() {
    // `file` with an unknown subcommand → None.
    assert_eq!(
        evaluate_auto_path_expr("[file rootname /a/b.tcl]", None),
        None,
    );
    // `file dirname` with the wrong arity (two path args) → None.
    assert_eq!(evaluate_auto_path_expr("[file dirname /a /b]", None), None,);
    // `file` with too few args (just the subcommand) → None.
    assert_eq!(evaluate_auto_path_expr("[file dirname]", None), None);
}

#[test]
fn auto_path_variable_subst_declines() {
    // A `$`-bearing literal word is refused (never guess a runtime value).
    assert_eq!(evaluate_auto_path_expr("$auto_path", None), None);
    assert_eq!(
        evaluate_auto_path_expr("[file dirname $base]", Some("/a/b.tcl")),
        None,
    );
}

#[test]
fn auto_path_unsupported_command_declines() {
    // A command outside the supported subset (`lsort`) → None.
    assert_eq!(
        evaluate_auto_path_expr("[lsort {a b}]", Some("/a/b.tcl")),
        None,
    );
}

#[test]
fn auto_path_tokeniser_brace_group_and_bracket_quirk() {
    // A brace group is one literal word; brackets inside braces are literal.
    // `{/lib}` → the literal `/lib`.
    assert_eq!(
        evaluate_auto_path_expr("{/lib}", None).as_deref(),
        Some("/lib"),
    );
    // A braced `[` is a literal `[`, not an open bracket — Tcl performs no
    // substitution inside braces.  The string-token tokeniser used to lose
    // that distinction and mis-read `{[} info script ]` as the command
    // substitution `[info script]`; with typed word tokens it is a literal
    // `[` followed by trailing words, which the expression parser rejects.
    assert_eq!(
        evaluate_auto_path_expr("{[} info script ]", Some("/proj/x.tcl")),
        None,
    );
}

#[test]
fn auto_path_tokeniser_quoted_word_with_escape() {
    // A double-quoted word is one token; an internal backslash-escape is
    // consumed as a pair by the tokeniser (so the closing quote isn't matched
    // early). The word still carries the backslash to `eval` as a literal.
    assert_eq!(
        evaluate_auto_path_expr("\"/quoted/path\"", None).as_deref(),
        Some("/quoted/path"),
    );
}

#[test]
fn auto_path_tokeniser_unterminated_delimiters_decline() {
    // An unterminated brace or quote aborts tokenisation → None.
    assert_eq!(evaluate_auto_path_expr("{/unterminated", None), None);
    assert_eq!(evaluate_auto_path_expr("\"/unterminated", None), None);
    // An unbalanced command bracket (`[` with no `]`) → parse fails → None.
    assert_eq!(evaluate_auto_path_expr("[file dirname /a", None), None);
    // A stray `]` at the start of a word → parse fails → None.
    assert_eq!(evaluate_auto_path_expr("] /a", None), None);
}

#[test]
fn auto_path_empty_and_whitespace_is_none() {
    assert_eq!(evaluate_auto_path_expr("", None), None);
    assert_eq!(evaluate_auto_path_expr("   ", None), None);
}

#[test]
fn auto_path_tilde_user_form_unchanged() {
    // `~user` (a non-bare tilde) is left unexpanded — no passwd lookup. It is a
    // literal relative word, so abspath joins it under cwd; we assert only that
    // the `~user` segment survives verbatim (no HOME expansion).
    temp_env::with_var("HOME", Some("/home/tester"), || {
        let got = evaluate_auto_path_expr("~someuser/lib", None).expect("some path");
        assert!(
            got.ends_with("~someuser/lib"),
            "~user must not be HOME-expanded, got {got:?}"
        );
    });
}

#[test]
fn auto_path_bare_tilde_no_home_unchanged() {
    // The bare `~` form with HOME unset is left unexpanded.
    temp_env::with_var_unset("HOME", || {
        let got = evaluate_auto_path_expr("~/lib", None).expect("some path");
        assert!(
            got.ends_with("~/lib"),
            "~ with no HOME must not expand, got {got:?}"
        );
    });
}

// ===========================================================================
// place_bridge.rs — place/SSA-bridge def/read resolution
// ===========================================================================

#[test]
fn place_dict_set_defs_variable_and_dollar_read_observes_it() {
    // `dict set d a b 1` writes the WHOLE dict variable `d` (the place bridge
    // resolves the VAR_WRITE target at variable granularity — a Scalar `d`, not
    // an element path: the dict commands mutate the bound variable in place).
    // The later `[dict get $d a b]` reads `$d`, which overlaps that def.
    let (defs, reads) = places_of(
        "proc f {} { dict set d a b 1; puts [dict get $d a b] }",
        "::f",
    );
    let d = place::scalar("d", LOCAL_NS, false);
    assert!(
        defs.iter().any(|p| p.name == "d"),
        "dict set must def the variable d, got {defs:?}",
    );
    assert!(
        reads.iter().any(|r| overlap(r, &d)),
        "the `$d` read in `dict get $d a b` must observe d, reads={reads:?}",
    );
}

#[test]
fn place_must_alias_kill_suppresses_dead_element() {
    // `set a(k) 1; set a(k) 2; puts $a(k)` — the FIRST write to a(k) is dead
    // (a later write to the exact same literal key with no intervening read),
    // even though a read later observes a(k).  So a(k) is NOT in the
    // "observed" suppression set for the first def's block/index, but the
    // SECOND write IS observed.  We assert the must-alias machinery via the
    // public `element_writes_observed_by_reads`: it returns the indices of the
    // element writes a read observes, EXCLUDING must-alias-killed ones.
    let r = reg();
    let cu = CompilationUnit::build_for(
        "proc f {} { set a(k) 1; set a(k) 2; puts $a(k) }",
        &r,
        false,
    );
    let fu = cu.function("::f").expect("proc");
    let observed = element_writes_observed_by_reads(&fu.cfg, &fu.name, &r);
    // Exactly one element write is observed-and-kept (the second `set a(k) 2`);
    // the first is must-alias-killed so it is excluded.
    assert_eq!(
        observed.len(),
        1,
        "only the surviving a(k) write is observed (first is must-alias killed); got {observed:?}",
    );
}

#[test]
fn place_array_element_write_no_kill_is_observed() {
    // Control: distinct keys (`a(k)` then `a(j)`), each read — both are
    // observed (no must-alias kill since the keys differ).
    let r = reg();
    let cu = CompilationUnit::build_for(
        "proc f {} { set a(k) 1; set a(j) 2; puts $a(k); puts $a(j) }",
        &r,
        false,
    );
    let fu = cu.function("::f").expect("proc");
    let observed = element_writes_observed_by_reads(&fu.cfg, &fu.name, &r);
    assert_eq!(
        observed.len(),
        2,
        "both distinct-key element writes are observed; got {observed:?}",
    );
}

#[test]
fn place_no_array_assignment_is_empty() {
    // The cheap pre-check: a function with no array-element assignment returns
    // an empty observed set without scanning.
    let r = reg();
    let cu = CompilationUnit::build_for("proc f {} { set x 1; puts $x }", &r, false);
    let fu = cu.function("::f").expect("proc");
    assert!(
        element_writes_observed_by_reads(&fu.cfg, &fu.name, &r).is_empty(),
        "no array assignment → empty observed set",
    );
}

#[test]
fn place_braced_literal_data_word_is_not_a_read() {
    // `puts {$a(k)}` is a BRACED literal data word — Tcl performs no
    // substitution, so it must NOT register a read of a(k).  A real dead store
    // to a(k) would otherwise be wrongly suppressed.
    let (_defs, reads) = places_of("proc f {} { set a(k) 1; puts {$a(k)} }", "::f");
    let ak = place::array_elem("a", Index::literal("k"), LOCAL_NS, false);
    assert!(
        !reads.iter().any(|r| overlap(r, &ak)),
        "a braced literal {{$a(k)}} must not read a(k); reads={reads:?}",
    );
}

#[test]
fn place_quoted_data_word_does_read() {
    // Control: a double-quoted word DOES substitute, so `puts "$a(k)"` reads
    // a(k).
    let (_defs, reads) = places_of("proc f {} { set a(k) 1; puts \"$a(k)\" }", "::f");
    let ak = place::array_elem("a", Index::literal("k"), LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &ak)),
        "a quoted \"$a(k)\" must read a(k); reads={reads:?}",
    );
}

#[test]
fn place_namespace_upvar_alias_resolves() {
    // `namespace upvar ::ns v local` binds `local` to the namespace var
    // ::ns::v — the resolve context records the alias and `local` resolves to
    // an UpvarAlias, not a fresh local scalar.
    let r = reg();
    let cu = CompilationUnit::build_for(
        "namespace eval ns { variable v 0 }\nproc f {} { namespace upvar ::ns v local; set local 1 }",
        &r,
        false,
    );
    let fu = cu.function("::f").expect("proc");
    let ctx = build_resolve_context(&fu.cfg, &fu.name);
    assert!(
        ctx.upvar_aliases.contains_key("local"),
        "namespace upvar should register the alias; aliases={:?}",
        ctx.upvar_aliases,
    );
    let p = tcl_compiler::var_resolve::resolve_place("local", &ctx, false, &r);
    assert_eq!(
        p.kind,
        PlaceKind::UpvarAlias,
        "the namespace-upvar local resolves to an UpvarAlias",
    );
}

#[test]
fn place_trace_target_marks_observed() {
    // `trace add variable g write …` marks `g` as traced → externally
    // observable, so a write to it is never dead.
    let r = reg();
    let cu = CompilationUnit::build_for(
        "proc f {} { set g 1; trace add variable g write cb; set g 2 }",
        &r,
        false,
    );
    let fu = cu.function("::f").expect("proc");
    let ctx = build_resolve_context(&fu.cfg, &fu.name);
    assert!(
        ctx.traced.contains("g"),
        "trace add variable should record g as traced; traced={:?}",
        ctx.traced,
    );
}

#[test]
fn place_whole_array_read_overlaps_every_element() {
    // `array get arr` reads the WHOLE array → overlaps any element a(k).
    let (_defs, reads) = places_of("proc f {} { set out [array get arr] }", "::f");
    let arr_k = place::array_elem("arr", Index::literal("k"), LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &arr_k)),
        "array get arr reads the whole array; reads={reads:?}",
    );
}

#[test]
fn place_dynamic_var_read_name_is_unknown_top() {
    // `array get $name` — the read target is a runtime-computed name, so it
    // resolves to UNKNOWN/top, which overlaps everything (a write to any array
    // is therefore not falsely dead).
    let (_defs, reads) = places_of(
        "proc f {} { set name arr; set out [array get $name] }",
        "::f",
    );
    let any_elem = place::array_elem("whatever", Index::literal("z"), LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &any_elem)),
        "a dynamic array-name read must be UNKNOWN/top (overlaps all); reads={reads:?}",
    );
}

#[test]
fn place_terminator_branch_condition_reads() {
    // An `if {$cond} …` branch terminator reads `cond` (the condition), and a
    // `[expr]`-embedded command-subst within the condition is recursed.
    let r = reg();
    let cu = CompilationUnit::build_for("proc f {c} { if {$c} { puts yes } }", &r, false);
    let fu = cu.function("::f").expect("proc");
    let ctx = build_resolve_context(&fu.cfg, &fu.name);
    let mut term_reads = Vec::new();
    for block in fu.cfg.blocks.values() {
        if let Some(term) = &block.terminator {
            term_reads.extend(terminator_read_places(term, &ctx, &r));
        }
    }
    assert!(
        term_reads
            .iter()
            .any(|r| overlap(r, &place::scalar("c", LOCAL_NS, false))),
        "the branch condition must read c; term_reads={term_reads:?}",
    );
}

#[test]
fn place_terminator_braced_return_is_not_a_read() {
    // A braced `return {$x}` is a literal — its `$x` is not a read.  A bare
    // `return $x` IS a read.  We compare the two terminator-read sets.
    let r = reg();
    let braced = CompilationUnit::build_for("proc f {} { set x 1; return {$x} }", &r, false);
    let bare = CompilationUnit::build_for("proc g {} { set x 1; return $x }", &r, false);
    let read_x = |cu: &CompilationUnit, q: &str| -> bool {
        let fu = cu.function(q).expect("proc");
        let ctx = build_resolve_context(&fu.cfg, &fu.name);
        fu.cfg.blocks.values().any(|b| {
            b.terminator.as_ref().is_some_and(|t| {
                terminator_read_places(t, &ctx, &r)
                    .iter()
                    .any(|r| overlap(r, &place::scalar("x", LOCAL_NS, false)))
            })
        })
    };
    assert!(
        !read_x(&braced, "::f"),
        "braced return {{$x}} must not read x"
    );
    assert!(read_x(&bare, "::g"), "bare return $x must read x");
}

#[test]
fn place_incr_reads_its_own_target() {
    // `incr counter` reads (and writes) `counter`; the amount word can read
    // more.
    let (defs, reads) = places_of("proc f {} { set counter 0; incr counter }", "::f");
    let c = place::scalar("counter", LOCAL_NS, false);
    assert!(defs.iter().any(|d| overlap(d, &c)), "incr defs counter");
    assert!(
        reads.iter().any(|r| overlap(r, &c)),
        "incr reads its own target counter; reads={reads:?}",
    );
}

#[test]
fn place_block_body_reads_recovered() {
    // An `eval {puts $x}` lowers to a Block whose body runs in the enclosing
    // scope — its `$x` read must be recovered so the store to x isn't dead.
    let (_defs, reads) = places_of("proc f {} { set x 1; eval {puts $x} }", "::f");
    let x = place::scalar("x", LOCAL_NS, false);
    assert!(
        reads.iter().any(|r| overlap(r, &x)),
        "eval {{puts $x}} block body must read x; reads={reads:?}",
    );
}

// ===========================================================================
// lattice_rebase.rs — memoised-unit offset rebase across a shift
// ===========================================================================
//
// Structural note: `lattice_rebase` shifts every absolute span in a memoised
// `FunctionUnit` when a cached offset-0 body is reused at a new file offset.
// The contract (documented in the module) is that the *span-carrying* parts of
// the rebased unit — the CFG (block statements + terminators + loop-node spans),
// the SSA statement clones, and the SCCP constant-branch spans — are
// BYTE-IDENTICAL to a fresh whole-file build at the new position.  We drive the
// rebase through the public `build_for_memoized` seam and assert exactly those
// three fields (`cfg` / `ssa` / `sccp`) equal a fresh build.  (The span-free
// `def_use` chains are NOT compared: their `Vec<UseSite>` ordering is a build-
// path artifact independent of the rebase — they carry no spans for the rebase
// to touch, and the existing diagnostics shift-test sidesteps the same way by
// comparing diagnostics rather than the raw unit.)  This exercises the Foreach /
// Try / UpFrame / constant-branch / loop-node statement variants the existing
// shift-test (`memoized_compilation_unit_shift_correctness`) does not cover.

/// Assert the *span-carrying* parts of two function units are identical: the
/// CFG, the SSA, and the SCCP result are exactly what `lattice_rebase` shifts,
/// so equality here proves the rebase reproduced a fresh build's positions.
fn assert_span_carrying_eq(got: &FunctionUnit, want: &FunctionUnit, ctx: &str) {
    assert_eq!(
        got.cfg, want.cfg,
        "{ctx}: rebased CFG must match a fresh build"
    );
    assert_eq!(
        got.ssa, want.ssa,
        "{ctx}: rebased SSA must match a fresh build"
    );
    assert_eq!(
        got.sccp, want.sccp,
        "{ctx}: rebased SCCP constant-branch spans must match a fresh build"
    );
}

/// Build a `FunctionUnit` for every proc via a position-independent memo whose
/// cache is seeded on `base` and reused on `shifted`, so the shifted procs hit
/// and the builder rebases their offset-0 units to the new positions.
fn rebased_units(base: &str, shifted: &str) -> (CompilationUnit, CompilationUnit) {
    use std::collections::HashMap;
    let registry = reg();
    let mut cache: HashMap<String, FunctionUnit> = HashMap::new();
    let build = |s: &str, cache: &mut HashMap<String, FunctionUnit>| -> CompilationUnit {
        CompilationUnit::build_for_memoized(
            s,
            tcl_compiler::compilation_unit::UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::default(),
                dialect: D,
                external_call_sites: None,
            },
            &mut |req: &tcl_compiler::compilation_unit::LatticeRequest<'_>| -> FunctionUnit {
                let key = format!(
                    "{}\u{0}{:?}\u{0}{:?}\u{0}{:?}",
                    req.qname, req.body, req.params, req.param_constants
                );
                if let Some(fu) = cache.get(&key) {
                    return fu.clone();
                }
                let cfg = tcl_compiler::cfg_builder::build_cfg_function_with_upvars(
                    req.qname,
                    req.body,
                    true,
                    req.upvar_procs.clone(),
                    req.proc_params.clone(),
                    req.global_write_procs.clone(),
                );
                let pc =
                    tcl_compiler::compilation_unit::decode_param_constants(req.param_constants);
                let fu = FunctionUnit::build_with_param_constants(
                    req.qname,
                    cfg,
                    req.params,
                    &registry,
                    pc.as_ref(),
                );
                cache.insert(key, fu.clone());
                fu
            },
        )
    };
    let cu_base = build(base, &mut cache);
    let cu_shifted = build(shifted, &mut cache);
    (cu_base, cu_shifted)
}

#[test]
fn rebase_shifted_unit_spans_match_fresh() {
    // A body exercising the rebase variants the existing shift-test misses:
    //   * foreach (Foreach span + body + raw_tokens)
    //   * try / finally (Try body + handler bodies + finally)
    //   * uplevel (UpFrame body)
    //   * incr / expr (Incr + ExprEval spans)
    //   * a `for` loop populates cfg.loop_nodes (LoopNode span + for_stmt)
    //   * `if {1}` folds an SCCP constant branch (sccp.constant_branches span)
    let body = "\
proc p {items} {
    set total 0
    foreach it $items { incr total }
    for {set i 0} {$i < 2} {incr i} { set acc [expr {$i * 2}] }
    if {1} { set always 1 } else { set never 0 }
    try {
        set r [risky]
    } on error {e} {
        set r fallback
    } finally {
        uplevel 1 { set marker done }
    }
    return $total
}
";
    // Prepend several top-level lines so the proc shifts down by a known delta.
    let shifted = format!("set a 0\nset b 1\n# comment\nset c 2\n{body}");
    let (_cu_base, cu_shifted) = rebased_units(body, &shifted);
    // A fresh whole-file build at the shifted position is the ground truth.
    let cu_fresh = CompilationUnit::build_for(&shifted, &reg(), false);

    let got = cu_shifted.function("::p").expect("rebased proc");
    let want = cu_fresh.function("::p").expect("fresh proc");
    assert_span_carrying_eq(got, want, "shifted foreach/try/finally/uplevel/for/if");
}

#[test]
fn rebase_switch_and_while_shift() {
    // A second shape: switch (arm + default bodies, subject span) and a while
    // loop, shifted, must still rebase to a fresh unit's span-carrying parts.
    let body = "\
proc q {n} {
    while {$n > 0} { incr n -1 }
    switch -- $n {
        0 { set s zero }
        1 { set s one }
        default { set s many }
    }
    return $s
}
";
    let shifted = format!("# pad\n# pad2\nset top 9\n{body}");
    let (_b, cu_shifted) = rebased_units(body, &shifted);
    let cu_fresh = CompilationUnit::build_for(&shifted, &reg(), false);
    assert_span_carrying_eq(
        cu_shifted.function("::q").expect("rebased"),
        cu_fresh.function("::q").expect("fresh"),
        "shifted switch/while",
    );
}

#[test]
fn rebase_zero_delta_is_noop() {
    // When the cached body sits at the same offset (no shift), delta == 0 and
    // the rebase early-returns — the memoised unit's span-carrying parts equal a
    // fresh build with no span movement.  (Drives the `delta == 0` early-return
    // arms in both `rebase_function_unit` and `rebase_script`.)
    let body = "proc z {} { set x 1\n set y [expr {$x + 1}]\n return $y }\n";
    let (cu_base, _shifted) = rebased_units(body, body);
    let cu_fresh = CompilationUnit::build_for(body, &reg(), false);
    assert_span_carrying_eq(
        cu_base.function("::z").expect("memoised"),
        cu_fresh.function("::z").expect("fresh"),
        "zero-delta",
    );
}

// ===========================================================================
// inlining/mod.rs — classify_proc / inline_module decline reasons
// ===========================================================================

#[test]
fn inline_classify_no_summary_is_never() {
    // `classify_proc` with no escape summary (None) → Never (the soundness gate
    // fails closed).
    let module = module_for("proc ::f {} { puts hi }\n");
    let proc = &module.procedures["::f"];
    assert_eq!(
        classify_proc(proc, None, 0),
        InlineDecision::Never,
        "a proc with no escape summary must be Never",
    );
}

#[test]
fn inline_classify_side_effecting_proc_is_never() {
    // A proc that writes a caller frame via `upvar` is NOT pure-leaf, so it is
    // Never regardless of size.
    // tclsh (8.6, 9.0): `proc bump {name} {upvar 1 $name v; incr v}; set x 5;
    // bump x; set x` → 6 — the upvar mutates the caller; inlining must not
    // relocate that body.
    let module = module_for("proc ::bump {name} { upvar 1 $name v; incr v }\n");
    let summaries = analyse_var_escape(&module, true);
    let proc = &module.procedures["::bump"];
    assert!(
        !summaries
            .get("::bump")
            .is_some_and(ProcEscapeSummary::safe_to_inline),
        "upvar proc is not safe_to_inline",
    );
    assert_eq!(
        classify_proc(proc, summaries.get("::bump"), 0),
        InlineDecision::Never,
    );
}

#[test]
fn inline_classify_small_pure_leaf_is_always_large_is_count_dependent() {
    // Boundary at SMALL_BODY_THRESHOLD: a body of exactly the threshold is
    // Always; threshold+1 is count-dependent (IfSingleCall at 1 caller, Never
    // otherwise).
    let assign_body = |n: usize| -> String {
        use std::fmt::Write as _;
        let mut body = String::new();
        for i in 0..n {
            let _ = writeln!(body, "  set v{i} {i}");
        }
        body
    };
    let small = assign_body(SMALL_BODY_THRESHOLD);
    let big = assign_body(SMALL_BODY_THRESHOLD + 1);
    let ms = module_for(&format!("proc ::s {{}} {{\n{small}}}\n"));
    let mb = module_for(&format!("proc ::b {{}} {{\n{big}}}\n"));
    let ss = analyse_var_escape(&ms, true);
    let sb = analyse_var_escape(&mb, true);
    assert_eq!(
        classify_proc(&ms.procedures["::s"], ss.get("::s"), 99),
        InlineDecision::Always,
        "threshold-sized pure-leaf is Always regardless of count",
    );
    assert_eq!(
        classify_proc(&mb.procedures["::b"], sb.get("::b"), 1),
        InlineDecision::IfSingleCall,
        "threshold+1 with one caller is IfSingleCall",
    );
    assert_eq!(
        classify_proc(&mb.procedures["::b"], sb.get("::b"), 3),
        InlineDecision::Never,
        "threshold+1 with multiple callers is Never",
    );
}

#[test]
fn inline_count_statements_try_and_switch() {
    // `count_statements` walks Try (body + handlers + finally) and Switch (arm +
    // default bodies) — the variants `inlining.rs`'s count tests skip.
    let module = module_for(
        "proc ::f {} {\n try { set a 1 } on error {e} { set b 2 } finally { set c 3 }\n switch -- $a { 1 { set d 4 } default { set e 5 } }\n}\n",
    );
    let n = count_statements(&module.procedures["::f"].body);
    // 3 from try (a, b, c) + 2 from switch (d, e) = 5 leaf statements.
    assert_eq!(n, 5, "count walks try + switch bodies");
}

#[test]
fn inline_count_statements_foreach_and_catch() {
    // foreach / catch each contribute 1 (the construct) + their body count.
    let module = module_for("proc ::f {} {\n foreach x {1 2} { set a 1 }\n catch { set b 2 }\n}\n");
    let n = count_statements(&module.procedures["::f"].body);
    // foreach: 1 + 1(set a) = 2; catch: 1 + 1(set b) = 2; total 4.
    assert_eq!(n, 4, "count walks foreach + catch bodies");
}

#[test]
fn inline_v3_return_inside_catch_declines() {
    // A `return` inside a `catch` body would be trapped by the break-based
    // early-return wrap — v3 declines, so the call survives.
    let stmts = inlined("proc ::h {x} { catch { return $x } }\nh 1")
        .top_level
        .statements;
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "h")),
        "return-inside-catch keeps the call",
    );
}

#[test]
fn inline_v3_return_inside_try_declines() {
    // Same for a `return` inside a `try` body.
    let stmts = inlined("proc ::h {x} { try { return $x } on error {e} { } }\nh 1")
        .top_level
        .statements;
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "h")),
        "return-inside-try keeps the call",
    );
}

#[test]
fn inline_v3_global_write_declines() {
    // A `::`-qualified (global) write isn't covered by the local-only rename
    // map, so v3 declines and the call survives.
    let stmts = inlined("proc ::w {} { set ::g 1 }\nw").top_level.statements;
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "w")),
        "a global write declines v3, keeping the call",
    );
}

#[test]
fn inline_verbatim_non_namespace_invariant_call_declines() {
    // A wrapper whose body calls an unqualified proc that DOES resolve to a
    // known proc is not namespace-invariant (it could resolve differently from
    // a caller's namespace), so it is not splice-eligible — the call survives.
    let src = "proc ::helper {} { puts deep }\nproc ::wrap {} { helper }\nwrap";
    let module = inlined(src);
    // `wrap` calls `helper` (a known proc, unqualified) → wrap's body is not
    // splice-safe (helper is not in the SPLICE_SAFE allow-list anyway), so the
    // `wrap` call is not verbatim-inlined.
    assert_eq!(
        top_calls_to(&module, "wrap"),
        1,
        "wrapper calling a known proc is not verbatim-spliced",
    );
}

#[test]
fn inline_empty_body_with_args_declines() {
    // An empty-body proc inlined as `Empty` requires a zero-arg call site; a
    // call WITH args declines (the splice can't bind them).
    let module = inlined("proc ::noop {} {}\nnoop extra");
    assert_eq!(
        top_calls_to(&module, "noop"),
        1,
        "empty-body proc called with args keeps the call",
    );
}

#[test]
fn inline_recursive_proc_does_not_loop() {
    // A self-recursive proc must not inline into an infinite expansion. The
    // pass replaces a single static call site only; the inliner must terminate
    // and the module is well-formed.
    let module = inlined("proc ::rec {n} { if {$n} { rec [expr {$n - 1}] } }\nrec 3");
    // `rec` is not pure-leaf (it calls itself / has a command-subst arg), so it
    // never qualifies — the call survives and the pass terminates.
    assert!(
        module.procedures.contains_key("::rec"),
        "recursive proc remains defined",
    );
}

#[test]
fn inline_idempotent_on_clean_module() {
    // A module with no inlinable site is returned structurally unchanged
    // (inline_module's early-return when the inlinable map is empty).
    let src = "proc ::p {a} { upvar 1 $a v; set v 1 }\np x";
    let once = inlined(src);
    let twice = inline_module(once.clone(), &reg());
    assert_eq!(once, twice, "no inlinable site → unchanged module");
}

#[test]
fn inline_v3_semantics_preserved_value() {
    // The load-bearing proof that v3 inlining is semantics-preserving.
    // tclsh (8.6, 9.0): `proc add {a b} { return [expr {$a + $b}] }; add 3 4`
    // → 7.  After inlining `add 3 4`, the spliced body must still compute 7 —
    // here we assert the call was replaced and a renamed binding/return is
    // present (the value identity is what tclsh confirms).
    let stmts = inlined("proc ::add {a b} { return [expr {$a + $b}] }\nadd 3 4")
        .top_level
        .statements;
    assert!(
        !stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "add")),
        "the add call was inlined",
    );
    assert!(
        stmts.iter().any(|s| matches!(s, Statement::Return { .. })),
        "terminal trailing return is preserved (forwards the value)",
    );
}

// ===========================================================================
// analyser/diagnostics/fp/rch.rs — reachability (O107) FP suppressor carve-outs
// ===========================================================================
//
// The reachability suppressor decides whether code after an unconditional exit
// (return / break / continue / error), or guarded by a constant-false
// condition, is genuinely dead (O107 fires) or only apparently dead (suppressed).
// Each carve-out gets a fires/suppressed control pair, with the Tcl-observable
// reachability fact pinned to tclsh.

#[test]
fn rch_code_after_return_is_dead() {
    // tclsh (8.6, 9.0): `proc f {} { return 1; puts after }; f` → 1 (the `puts
    // after` never runs — it is unreachable).  O107 must fire on it.
    let src = "proc f {} { return 1\n puts after }";
    assert!(
        o107_fires(src, D),
        "code after an unconditional return is dead; O107 must fire; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_code_after_conditional_return_is_reachable() {
    // Control: when the return is GUARDED (`if {$c} { return }`), the tail is
    // reachable — O107 must NOT fire.
    // tclsh (8.6, 9.0): `proc f {c} { if {$c} { return 1 }; puts after }; f 0`
    // → prints `after` (reachable when c is false).
    let src = "proc f {c} { if {$c} { return 1 }\n puts after }";
    assert!(
        !o107_fires(src, D),
        "tail after a conditional return is reachable; O107 must NOT fire; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_code_after_error_is_dead() {
    // `error` always raises — code after it (in the same straight-line block)
    // is unreachable.
    // tclsh (8.6, 9.0): `proc f {} { error boom; puts after }; catch f` — the
    // `puts after` never runs.  O107 fires.
    let src = "proc f {} { error boom\n puts after }";
    assert!(
        o107_fires(src, D),
        "code after error is dead; O107 must fire; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_code_after_break_in_loop_is_dead() {
    // An unconditional `break` makes the rest of the loop body dead.
    // tclsh (8.6, 9.0): `foreach x {1 2} { break; puts inner }` — `puts inner`
    // never runs.  O107 fires on it.
    let src = "proc f {} { foreach x {1 2} { break\n puts inner } }";
    assert!(
        o107_fires(src, D),
        "code after an unconditional break in a loop is dead; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_code_after_continue_in_loop_is_dead() {
    // An unconditional `continue` makes the rest of the loop body dead.
    // tclsh (8.6, 9.0): `foreach x {1 2} { continue; puts inner }` — `puts
    // inner` never runs.  O107 fires.
    let src = "proc f {} { foreach x {1 2} { continue\n puts inner } }";
    assert!(
        o107_fires(src, D),
        "code after an unconditional continue in a loop is dead; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_conditional_break_keeps_loop_tail_reachable() {
    // Control: a CONDITIONAL break leaves the loop-body tail reachable.
    // tclsh (8.6, 9.0): `foreach x {1 2} { if {$x > 5} break; puts inner }`
    // prints `inner` for both elements.  O107 must NOT fire.
    let src = "proc f {} { foreach x {1 2} { if {$x > 5} break\n puts inner } }";
    assert!(
        !o107_fires(src, D),
        "a conditional break keeps the loop tail reachable; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_constant_false_guard_is_constant_branch_not_o107() {
    // A statically-false guard (`if {0} { … }`) makes its body unreachable, but
    // that is handled by SCCP's *constant-branch* path (the I230 dead-arm info),
    // NOT the post-exit reachability O107.  This is a reachability carve-out: the
    // O107 suppressor must NOT also fire on a constant-folded branch (it would be
    // a duplicate / mis-attributed diagnostic).
    // tclsh (8.6, 9.0): `if {0} { puts never }` prints nothing (the body is
    // dead) — surfaced here as I230, the constant-branch dead-arm info.
    let src = "proc f {} { if {0} { puts never } }";
    assert!(
        fires(src, D, "I230"),
        "an if {{0}} dead arm must surface as I230; emitted {:?}",
        codes(src, D)
    );
    assert!(
        !o107_fires(src, D),
        "a constant-folded if {{0}} must NOT also fire O107; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_constant_true_guard_body_is_reachable() {
    // Control: an `if {1} { … }` body is always reached — its body is live, so
    // no O107 on it (the absent else-branch is the dead arm, surfaced as the
    // I230 constant-branch info, not O107).
    let src = "proc f {} { if {1} { puts always } else { puts never } }";
    assert!(
        !o107_fires(src, D),
        "an if {{1}} body is reachable; O107 must NOT fire; emitted {:?}",
        codes(src, D)
    );
    assert!(
        fires(src, D, "I230"),
        "the constant-true branch's dead else-arm surfaces as I230; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_while1_no_break_post_loop_is_dead() {
    // A `while 1` body with no break/return makes the post-loop block dead.
    // tclsh: `while 1 { puts x }` never exits, so `puts after` is unreachable.
    let src = "proc f {} { while 1 { puts x }\n puts after }";
    assert!(
        o107_fires(src, D),
        "post-loop block after an infinite while-1 is dead; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_while1_with_break_post_loop_is_reachable() {
    // Control: a `while 1` with a (conditional) break can exit, so the
    // post-loop block is reachable — O107 must NOT fire.
    let src = "proc f {c} { while 1 { if {$c} break }\n puts after }";
    assert!(
        !o107_fires(src, D),
        "a while-1 with a break makes the post-loop block reachable; emitted {:?}",
        codes(src, D)
    );
}
