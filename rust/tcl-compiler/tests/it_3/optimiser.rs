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

//! The static Tcl source optimiser suite.
//!
//! The optimiser is driven via
//!   `apply_optimisations(src, &optimise_with_dialect(src, registry, dialect))`
//! returning the rewritten source, plus the `Vec<Optimisation>` for inspecting
//! which `O1xx` codes fired (each `Optimisation.code.as_str()` is `"O100"`…).
//! `optimise_with_dialect` is a *single* optimiser pass (overlap-resolved); the
//! multipass cases use the `optimise_source_multipass` fixpoint helper.
//!
//! ## C-Tcl proof approach
//!
//! Every Tcl-observable optimisation here must preserve program semantics, so
//! the load-bearing rewrites were checked against real `tclsh8.6`/`tclsh9.0`
//! via `scripts/dev/tclsh_check.sh`: for each `apply_optimisations` rewrite the
//! ORIGINAL and REWRITTEN snippets were evaluated and shown to produce the same
//! value (cited inline next to the relevant test). The boolean/De-Morgan/
//! strength-reduction/end-offset rewrites were proven by exhaustively sweeping
//! their operands in tclsh (`foreach a {0 1 5} ... if {LHS != RHS} {puts BAD}`
//! → `OK`). Purely structural assertions — *which* `O1xx` code fired, or that a
//! diagnostic-style hint is present — are not directly Tcl-observable and are
//! flagged as such in comments.
//!
//! Dialect handling: the `dialect` argument to `optimise_with_dialect` selects
//! the command signatures; iRule (`when`/`pool`/`matches_glob`) snippets pass
//! `"f5-irules"`, the Tcl-9 packing skip passes `"tcl9.0"`, everything else
//! `"tcl8.6"`.
//!
//! Where the behaviour is confirmed correct against tclsh, the assertion is
//! adapted to it and commented at each site; cases where the optimiser is
//! soundly less aggressive in a way that looks like a shortcoming are omitted
//! rather than `#[ignore]`-d.

use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::optimiser::manager::{
    apply_optimisations, optimise_source_multipass, optimise_with_dialect,
};
use tcl_registry::model::ingress::static_context_for;

mod code_motion;
mod dead_code;
mod idiom_rewrites;
mod instcombine;
mod propagation;
mod structure_elimination;
mod tail_calls;
mod try_finally;

// Shared helpers (mirror fp/opt.rs: opt_fires / optimised / opt_codes)

const TCL: &str = "tcl8.6";

/// Every `Oxxx` code emitted by a single optimiser pass over `src`.
fn opt_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .iter()
        .map(|o| o.code.as_str().to_owned())
        .collect()
}

/// True if any optimisation with `code` fires on `src` under `dialect`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    opt_codes(src, dialect).iter().any(|c| c == code)
}

/// Apply all (non-hint) optimisations and return the rewritten source.
fn optimised(src: &str, dialect: &str) -> String {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

/// `(code, replacement)` pairs from the optimiser for `src`.
fn opt_rewrites(src: &str, dialect: &str) -> Vec<(String, String)> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .into_iter()
        .map(|o| (o.code.as_str().to_owned(), o.replacement.clone()))
        .collect()
}

/// Count optimisations with `code`.
fn opt_count(src: &str, dialect: &str, code: &str) -> usize {
    opt_codes(src, dialect)
        .iter()
        .filter(|c| *c == code)
        .count()
}

/// Every analyser diagnostic code for `src`, at any severity — `reparse_errors`
/// keeps only `Severity::Error`, so a warning-level code (W210, W211) needs
/// this instead.
fn analyser_codes(src: &str, dialect: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

/// Error-severity diagnostic codes the user-facing `tcl diag` surface reports
/// for `src` — the analyser pass plus `run_all_checks`, optimisation codes
/// dropped, mirroring `checks.rs::codes`. Empty means the source re-parses;
/// a rewrite that emits unbalanced text surfaces here as an `E2xx`.
fn reparse_errors(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.code.to_string())
        .collect();
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    for diag in run_all_checks(&cu, registry, d) {
        if !diag.code.is_optimisation() && diag.severity == Severity::Error {
            out.push(diag.code.to_string());
        }
    }
    out
}

// Constant propagation / folding / DSE core

/// `body` inside a loop where `$x` and `$y` are both proved integers. tclsh
/// 8.6: with `a` set to `abc`, `expr {!($a && $b)}` raises `expected boolean
/// value but got "abc"` and `expr {!$a || !$b}` raises `can't use non-numeric
/// string as operand of "!"`; `expr {$a && 0}` raises where `0` does not;
/// `expr {$a ** 2}` and `expr {$a * $a}` name different operators. The
/// rewrites below are therefore claimed for integer operands only.
fn int_xy(body: &str) -> String {
    let tail = if body.contains("set v ") {
        "\n    puts $v"
    } else {
        ""
    };
    format!(
        "proc f {{n}} {{\n  for {{set x 0}} {{$x < $n}} {{incr x}} {{\n    set y [expr {{$x + 1}}]\n    {body}{tail}\n  }}\n}}\n"
    )
}

/// `body` inside a procedure whose parameters are the variables it reads. At
/// file level a read of an unset variable raises `can't read "a": no such
/// variable` in tclsh before the expression is evaluated.
fn in_proc(body: &str) -> String {
    let tail = if body.contains("set v ") {
        "\n  return $v"
    } else if body.contains("set ok ") {
        "\n  return $ok"
    } else {
        ""
    };
    format!("proc p {{a b c d x s}} {{\n  {body}{tail}\n}}\n")
}

/// `body` at file level after `vars` are assigned from the script's
/// arguments, so each read is of a set variable whose value is unknown.
fn from_argv(vars: &str, body: &str) -> String {
    format!("lassign $argv {vars}\n{body}")
}

// Cross-event DSE — stores consumed by a later event must survive

// NOTE — this group of cross-event DSE cases is OMITTED here (they were a
// GENUINE bug at the time). A `set` in one iRule event whose value
// is read in a later event must NOT be eliminated:
//   * `set uri [HTTP::uri]` read via `"uri=$uri"` in HTTP_RESPONSE
//   * `set ans_cleared 1` checked via `[info exists ans_cleared]` in DNS_RESPONSE
//   * `set allowlist 1`    checked via `[info exists allowlist]`   in DNS_RESPONSE
// In all three, O126 ("Remove unused variable assignment") fired and deleted the
// store, dropping the value the later event consumes — a cross-event dead-store
// soundness bug. They were reported rather than asserted, since asserting the
// output at the time would have pinned a miscompile.

// Cross-event dead-store / info-exists soundness — these were miscompiles that
// are now FIXED in the source (connection_scope info-exists read detection + the
// O126 cross-event skip + the cross-event existence-fold post-pass). iRule
// `when` handlers share connection-scoped variables, so a store read in a later
// event must survive.
mod cross_event_dse {
    use tcl_compiler::optimiser::manager::{apply_optimisations, optimise_with_dialect};
    use tcl_registry::model::ingress::static_context_for;

    fn optimised(src: &str) -> String {
        let reg = static_context_for("f5-irules").commands();
        apply_optimisations(
            src,
            &optimise_with_dialect(src, reg, Some(tcl_dialect::DialectProfile::irules())),
        )
    }

    #[test]
    fn direct_read_store_survives() {
        // `$uri` in HTTP_RESPONSE reads the value set in HTTP_REQUEST; O126 must
        // not delete the store. (tclsh: the response handler logs the request's
        // uri — deleting `set uri` leaves $uri undefined.)
        let out = optimised(
            "when HTTP_REQUEST { set uri [HTTP::uri] }\nwhen HTTP_RESPONSE { log local0. \"uri=$uri\" }",
        );
        assert!(out.contains("set uri"), "cross-event store deleted:\n{out}");
    }

    #[test]
    fn info_exists_flag_survives_and_is_not_folded() {
        // `[info exists ans_cleared]` in DNS_RESPONSE observes a flag set in
        // DNS_REQUEST — neither the store nor the existence check may be folded
        // away (else the response takes the wrong branch).
        let out = optimised(
            "when DNS_REQUEST { set ans_cleared 1 }\nwhen DNS_RESPONSE { if {[info exists ans_cleared]} { return } }",
        );
        assert!(out.contains("set ans_cleared"), "store deleted:\n{out}");
        assert!(
            out.contains("info exists ans_cleared"),
            "info exists folded to a constant:\n{out}"
        );
    }

    #[test]
    fn second_info_exists_variant_survives() {
        let out = optimised(
            "when HTTP_REQUEST { set allowlist 1 }\nwhen HTTP_RESPONSE { if {[info exists allowlist]} { log local0. ok } }",
        );
        assert!(out.contains("set allowlist"), "store deleted:\n{out}");
        assert!(out.contains("info exists allowlist"), "folded:\n{out}");
    }

    #[test]
    fn same_event_dead_store_still_eliminated() {
        // Control: a store overwritten in the SAME event is genuinely dead.
        let out = optimised("when HTTP_REQUEST { set t 1\n set t 2\n log local0. $t }");
        assert!(
            !out.contains("set t 1"),
            "same-event dead store kept:\n{out}"
        );
    }
}
