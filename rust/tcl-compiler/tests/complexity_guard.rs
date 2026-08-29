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

//! Deep-analysis complexity guard (Phase 6).
//!
//! ## What the guard does
//!
//! A pathologically large (machine-generated) body skips deep analysis (SSA /
//! dataflow / analyser-walk / compiler-checks / shimmer / taint / optimiser /
//! interproc) so a single giant generated proc can't cost tens of seconds. The
//! guard is sized well above legitimate hand-written code, so normal procs are
//! unaffected. There are two independent halves:
//!
//!   * **block-count** — [`tcl_compiler::ssa::COMPLEXITY_GUARD_BLOCKS`] (`20_000)`:
//!     applied by [`tcl_compiler::ssa::is_complexity_guarded`] over the CFG, and
//!     as a backstop inside `FunctionUnit::build_*`.
//!   * **body-bytes** — [`tcl_compiler::ssa::DEEP_ANALYSIS_BODY_BYTES`]
//!     (256 KiB): applied by the compilation-unit builder, which has the body
//!     span; a flat generated command list is block-light (so the block half
//!     never fires) yet byte-huge.
//!
//! Both halves set the single `FunctionUnit::complexity_guarded` flag and yield
//! a `trivial_guarded` unit (empty SSA, empty dataflow lattices). Per-proc
//! diagnostic / optimiser passes consult the flag (via
//! `CompilationUnit::analysable_functions`) and skip the function entirely, so
//! the empty lattices are never read as real facts.
//!
//! ## Observation surfaces
//!
//! These are compiler-internal *structural* facts, so each is asserted directly
//! on the crate's public API — no `tclsh` ground truth is involved.
//!
//!   * The **block-count** half is exercised on a hand-rolled `cfg::Function`
//!     chain, calling `is_complexity_guarded` / `build_ssa` directly:
//!     [`chain_cfg`] builds a straight-line chain as a
//!     [`tcl_compiler::cfg::Function`], and the assertions use
//!     `is_complexity_guarded` + `build_ssa`.
//!   * The **byte** half is observed through diagnostics (`W123` suppression)
//!     and through `CompilationUnit::build_for`; the per-proc unit it yields
//!     (`cu.function("::big")`) carries the `complexity_guarded` /
//!     `ssa` / `analysis` bundle. The W123
//!     diagnostic-suppression surface is not re-derived here (it is an effect of
//!     the same flag, which is asserted directly); see the GAP note on the
//!     analyser-walk case.
//!
//! ## Divergences
//!
//!   * There is no `force_guard=True` keyword on `build_ssa` / the analysis
//!     primitives — the byte-half short-circuit is instead
//!     realised by the builder choosing `FunctionUnit::trivial_guarded` *before*
//!     calling `build_ssa` at all (so there is no expensive walk to skip). The
//!     observable end-state (a byte-huge but block-light proc gets a
//!     trivial unit) is covered by [`byte_guarded_proc_skips_ssa_and_dataflow`].
//!     See the `// GAP:`-tagged `force_guard_*` placeholders.

use tcl_compiler::cfg::{Block, Function, Terminator};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::ssa::{
    COMPLEXITY_GUARD_BLOCKS, DEEP_ANALYSIS_BODY_BYTES, build_ssa, is_complexity_guarded,
};
use tcl_registry::CommandRegistry;
use tcl_registry::model::ingress::static_context_for;

const TCL: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    static_context_for(TCL).commands()
}

/// A straight-line chain of `n` blocks (`b0 → b1 → … → return`). Built directly
/// on the public
/// [`Function`] API (`new` seeds an empty `b0` entry; `intern_block` +
/// `blocks.insert` add the rest), so it can be made arbitrarily large without
/// going through lowering/codegen.
fn chain_cfg(n: usize) -> Function {
    assert!(n >= 1, "chain needs at least the entry block");
    // `Function::new` interns `b0` as the entry and inserts an empty block for
    // it; we overwrite b0's terminator below to point at b1 (when n > 1).
    let mut f = Function::new("::big", "b0");
    for i in 0..n {
        let name = format!("b{i}");
        let id = f.intern_block(name.clone());
        let terminator = if i < n - 1 {
            // Goto the next block in the chain.
            let target = f.intern_block(format!("b{}", i + 1));
            Terminator::Goto { target, span: None }
        } else {
            Terminator::Return {
                value: None,
                span: None,
                expr: None,
                braced: false,
            }
        };
        let mut block = Block::new(name);
        block.terminator = Some(terminator);
        f.blocks.insert(id, block);
    }
    assert_eq!(f.blocks.len(), n, "chain_cfg built exactly n blocks");
    f
}

// The block-count half of the guard.

/// A body well below the ceiling is
/// not guarded and gets a real SSA (one SSA block per CFG block).
#[test]
fn below_threshold_is_analysed() {
    let cfg = chain_cfg(100); // well below the ceiling
    assert!(!is_complexity_guarded(&cfg));
    let ssa = build_ssa(&cfg, registry());
    assert_eq!(ssa.blocks.len(), cfg.blocks.len()); // real SSA
}

/// A body just over the ceiling is guarded, and `is_complexity_guarded` is what
/// decides it.
///
/// `build_ssa(func, registry)` is the raw, always-builds primitive — the guard
/// is applied one level up by the
/// `FunctionUnit` / `CompilationUnit` builders (which substitute
/// `trivial_guarded`), not inside `build_ssa`. So here we assert the *decision*
/// (`is_complexity_guarded`) and leave the "trivial unit" assertion to the
/// `CompilationUnit`-level byte-half tests below, which exercise the real
/// guarded path end-to-end. Building a full 20_005-block SSA just to discard it
/// would also be the exact cost the guard exists to avoid.
#[test]
fn above_threshold_is_guarded() {
    let cfg = chain_cfg(COMPLEXITY_GUARD_BLOCKS + 5);
    assert!(is_complexity_guarded(&cfg));
}

/// The boundary is strict `>` (a body of exactly `COMPLEXITY_GUARD_BLOCKS`
/// blocks is still analysed; `+ 1` trips it). Pins the off-by-one so a refactor
/// of `is_complexity_guarded` can't silently flip the ceiling to `>=`.
#[test]
fn block_guard_boundary_is_strict_greater_than() {
    let at_ceiling = chain_cfg(COMPLEXITY_GUARD_BLOCKS);
    assert!(
        !is_complexity_guarded(&at_ceiling),
        "exactly COMPLEXITY_GUARD_BLOCKS blocks is NOT guarded"
    );
    let over = chain_cfg(COMPLEXITY_GUARD_BLOCKS + 1);
    assert!(
        is_complexity_guarded(&over),
        "one block over the ceiling IS guarded"
    );
}

/// A block-light CFG built straight from `Function::new` (1 block) is never
/// guarded by the block half — the lower bound of the `chain_cfg` helper and a
/// sanity check that an empty/tiny function is analysed.
#[test]
fn tiny_function_is_not_block_guarded() {
    let one = chain_cfg(1);
    assert_eq!(one.blocks.len(), 1);
    assert!(!is_complexity_guarded(&one));
}

// The body-byte half of the guard.
//
// The byte half is applied by `DEEP_ANALYSIS_BODY_BYTES` in
// `CompilationUnit::build_for_with_config` over the proc's body span. These
// tests exercise it end-to-end through `build_for`.

/// One statement carrying a ~270 KB string literal: block-light (the block half
/// would NOT fire) but byte-huge. Returns the source text.
fn byte_huge_proc_src() -> String {
    // 270_000 > DEEP_ANALYSIS_BODY_BYTES (262_144); one `set` statement, so the
    // CFG is a couple of blocks at most.
    let big_literal = "A".repeat(270_000);
    format!("proc big {{}} {{ set x \"{big_literal}\" }}")
}

/// A normal hand-written proc is NOT guarded and gets a real
/// SSA + SCCP. This is the "normal procs are unaffected" invariant.
#[test]
fn normal_proc_is_not_guarded() {
    let cu = CompilationUnit::build_for("proc small {} { set x 1 }", registry(), false);
    let small = cu.function("::small").expect("::small built");
    assert!(!small.complexity_guarded);
    assert!(!small.ssa.blocks.is_empty(), "real SSA for a normal proc");
    // A genuine fact is recorded for the normal proc (proof deep analysis ran):
    // SCCP folds the `set x 1` literal, so `values` is non-empty.
    assert!(
        !small.sccp.values.is_empty(),
        "normal proc has real dataflow facts"
    );
    // Not excluded from the analysable-function view.
    assert!(
        cu.analysable_functions().any(|fu| fu.name == "::small"),
        "normal ::small is analysed (not skipped)"
    );
}

/// A block-light but
/// byte-huge proc gets a trivial `FunctionUnit` (empty SSA + no facts) — the
/// byte guard is decided *before* SSA/dataflow, so the O(blocks·vars) walk and
/// the SCCP/taint/liveness passes never run on it.
#[test]
fn byte_guarded_proc_skips_ssa_and_dataflow() {
    let cu = CompilationUnit::build_for(&byte_huge_proc_src(), registry(), false);
    let big = cu.function("::big").expect("::big built");

    // Block-light: the block-count half would NOT fire on this body ...
    assert!(
        !is_complexity_guarded(&big.cfg),
        "byte-huge body is block-light — the block half must not fire"
    );
    // ... yet it is guarded (byte-heavy) and its SSA/dataflow were skipped.
    assert!(
        big.complexity_guarded,
        "byte-huge body must be guarded by the byte half"
    );
    assert!(big.ssa.blocks.is_empty(), "guarded unit has trivial SSA");
    assert!(big.sccp.values.is_empty(), "guarded unit has no SCCP facts");
    assert!(big.types.is_empty(), "guarded unit has no type facts");
    assert!(big.taints.is_empty(), "guarded unit has no taint facts");
}

/// The proc itself is still recorded
/// (resolves as a command / built unit) — only its body is left un-analysed. A
/// guarded unit is still a *valid trivial* `CompilationUnit` member.
#[test]
fn guarded_proc_still_recorded() {
    let cu = CompilationUnit::build_for(&byte_huge_proc_src(), registry(), false);
    // The procedure is present in the unit even though its body was skipped.
    let big = cu.function("::big").expect("::big still recorded");
    assert_eq!(big.name, "::big");
    assert!(big.complexity_guarded);
    // The trivial unit is well-formed: its SSA carries the CFG's block-name
    // interner and entry, so a BlockId still resolves to a name (no panic) and
    // the empty lattices are valid (just facts-free).
    assert_eq!(big.ssa.entry, big.cfg.entry);
    assert_eq!(big.ssa.block_names().len(), big.cfg.block_names().len());
}

/// A guarded function is dropped from the `analysable_functions` view the
/// per-proc diagnostic / optimiser passes iterate, so the empty lattices are
/// never consumed as real facts (the W123-suppression effect, asserted at its
/// structural source).
#[test]
fn guarded_proc_excluded_from_analysable_functions() {
    let cu = CompilationUnit::build_for(&byte_huge_proc_src(), registry(), false);
    assert!(
        cu.function("::big").is_some(),
        "guarded ::big exists as a unit",
    );
    assert!(
        cu.analysable_functions().all(|fu| fu.name != "::big"),
        "guarded ::big must be skipped by analysable_functions"
    );
}

/// The guarded path must not panic or hang. We additionally assert it's well
/// under a generous 20 s budget — the whole point of the guard is that the
/// byte-huge body is *cheap* because deep analysis is skipped.
#[test]
fn huge_body_does_not_crash_and_is_fast() {
    let src = byte_huge_proc_src();
    let t = std::time::Instant::now();
    let cu = CompilationUnit::build_for(&src, registry(), false); // must not panic
    let big = cu.function("::big").expect("::big built");
    assert!(big.complexity_guarded);
    assert!(
        t.elapsed().as_secs() < 20,
        "guarded path must be fast (deep analysis skipped)"
    );
}

/// The byte boundary uses the proc's body *span*, not its CFG block count: a
/// proc whose literal is comfortably *below* `DEEP_ANALYSIS_BODY_BYTES` is
/// analysed (not byte-guarded). Pins that a sub-ceiling body is not over-guarded
/// — the dual of `byte_guarded_proc_skips_ssa_and_dataflow`.
#[test]
fn sub_ceiling_body_is_not_byte_guarded() {
    // ~1 KiB literal — three orders of magnitude under the 256 KiB ceiling.
    let small_literal = "A".repeat(1_024);
    assert!(small_literal.len() < DEEP_ANALYSIS_BODY_BYTES);
    let src = format!("proc modest {{}} {{ set x \"{small_literal}\" }}");
    let cu = CompilationUnit::build_for(&src, registry(), false);
    let modest = cu.function("::modest").expect("::modest built");
    assert!(
        !modest.complexity_guarded,
        "a sub-ceiling body must NOT be byte-guarded"
    );
    assert!(!modest.ssa.blocks.is_empty(), "real SSA for a modest proc");
}

// Force-guard short-circuit — API surface with no analogue here.
//
// GAP: there is no `force_guard=True` keyword on `build_ssa(func, registry)` /
// the analysis primitives. The byte-half
// short-circuit is realised structurally: the compilation-unit
// builder selects `FunctionUnit::trivial_guarded(...)` *before* calling
// `build_ssa`, so there is no expensive walk to force-skip — `build_ssa` is
// simply never invoked for a guarded body. The observable end-state
// (a body the caller judged too large gets an empty
// SSA + facts-free analysis) is therefore covered by
// `byte_guarded_proc_skips_ssa_and_dataflow` and `guarded_proc_still_recorded`
// above. A forced short-circuit has no direct analogue here and is not
// covered (see the module-level divergence note).

/// Documents (and pins) that the block-half decision is the
/// `is_complexity_guarded` predicate over a small CFG: a 3-block chain
/// is not block-guarded, so
/// the only way it becomes trivial is the *forced* byte-half path — expressed
/// via `trivial_guarded`, not a `force_guard` flag.
#[test]
fn force_guard_baseline_small_cfg_is_not_block_guarded() {
    // Shared precondition of the forced short-circuit
    // (`is_complexity_guarded` is false for a 3-block chain); the forced-trivial
    // behaviour itself has no `force_guard` analogue (see note above).
    let cfg = chain_cfg(3);
    assert!(!is_complexity_guarded(&cfg));
    // Full SSA by default (blocks match the CFG one-to-one).
    let ssa = build_ssa(&cfg, registry());
    assert_eq!(ssa.blocks.len(), cfg.blocks.len());
}
