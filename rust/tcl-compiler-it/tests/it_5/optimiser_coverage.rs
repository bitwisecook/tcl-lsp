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

//! The comprehensive optimiser *coverage* suite (O100–O129, the shimmer
//! S100–S102 pipeline, and the GVN/CSE/LICM redundancy detector).
//!
//! ## Driving the optimiser
//!
//! Three entry points are exercised:
//!   * `apply_optimisations(src, &optimise_with_dialect(...))` for the rewritten
//!     text, and the `Vec<Optimisation>` for the `O1xx` codes.
//!   * the explicitly legacy CFG/SSA GVN finder, for value-numbering algorithm
//!     coverage over a `CompilationUnit`. Production GVN instead requires exact
//!     common-semantic invocation facts and is covered at that boundary in the
//!     `gvn` module tests.
//!   * the shimmer/thunking detector: `find_shimmer_warnings_for_cu` (S100/S101)
//!     + `find_thunking_warnings_for_cu` (S102).
//!
//! De-Morgan / inversion logic lives inside the O110 `InstCombine` pass on the
//! expr AST (there is no standalone string→string helper). Those cases are
//! exercised as O110-driven `[expr {...}]` rewrites, the form `optimiser.rs`
//! already uses.
//!
//! ## C-Tcl proof
//!
//! Every assertion that encodes a Tcl-language *value* fact (a fold result, an
//! arithmetic outcome, an evaluation result) was checked against `tclsh9.0`
//! (and the De-Morgan / strength-reduction / self-comparison families were
//! exhaustively swept over their operands), with the verified value cited in a
//! `// tclsh:` comment. Optimiser-structural facts (which `O1xx`/`S1xx` code
//! fired, that a store was eliminated) are about the compiler IR, not Tcl
//! semantics, and are flagged as such.
//!
//! ## Notes on behaviour
//!
//! Where the optimiser is confirmed *sound* against tclsh (more aggressive, or a
//! different but equivalent O-code), the assertion is adapted to the actual
//! behaviour and commented at the site. Gaps (the optimiser soundly leaves a
//! construct unchanged) assert the conservative no-op. A genuine miscompile
//! (`$x - $x` → 0 on an untyped operand) has since been FIXED (O110 now gates the
//! self-subtraction/xor fold on a provably-numeric operand); the assertion
//! asserts the corrected behaviour. No `#[ignore]`; every test passes.

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::gvn::find_redundancies;
use tcl_compiler::optimiser::manager::{
    apply_optimisations, optimise_source_multipass, optimise_with_dialect,
};
use tcl_compiler::shimmer::{find_shimmer_warnings_for_cu, find_thunking_warnings_for_cu};
use tcl_registry::model::ingress::static_context_for;

mod code_motion;
mod constant_folding;
mod dead_code;
mod idiom_rewrites;
mod instcombine;
mod procedures;
mod robustness;
mod shimmer;
mod store_forwarding;

const TCL: &str = "tcl8.6";
const IR: &str = "f5-irules";

// Helpers (mirror optimiser.rs: opt_codes / opt_fires / optimised)

/// Sorted, de-duplicated `Oxxx` codes emitted by a single optimiser pass.
fn opt_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    let mut v: Vec<String> = optimise_with_dialect(src, registry, d)
        .iter()
        .map(|o| o.code.as_str().to_owned())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// True if any optimisation with `code` fires on `src` under `dialect`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code.as_str() == code)
}

/// True if NONE of the optimisations carry `code`.
fn opt_absent(src: &str, dialect: &str, code: &str) -> bool {
    !opt_fires(src, dialect, code)
}

/// Apply all optimisations and return the rewritten source.
fn optimised(src: &str, dialect: &str) -> String {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

/// True if any emitted code is in `wanted`.
fn opt_any_of(src: &str, dialect: &str, wanted: &[&str]) -> bool {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| wanted.contains(&o.code.as_str()))
}

/// Wraps `body` in a loop where `$x` is SCCP-typed INT and live (the
/// precondition the D5-O110 identity/annihilator drops require). Matches the
/// helper in `optimiser.rs`.
fn int_x(body: &str) -> String {
    format!(
        "proc f {{n}} {{\n  for {{set x 0}} {{$x < $n}} {{incr x}} {{\n    {body}\n    puts $v\n  }}\n}}\n"
    )
}

// O100 — constant propagation into expressions

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
