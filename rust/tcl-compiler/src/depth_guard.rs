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

//! Shared native-stack depth caps for this crate's two unbounded
//! recursive-descent categories — issue #996.
//!
//! Both walkers below recurse *independently* of the crate's existing
//! `Script`/`Statement`-tree caps (`analyser::commands::MAX_BODY_DEPTH`,
//! `lowering::MAX_LOWER_NEST_DEPTH`, `optimiser::MAX_OPTIMISER_WALK_DEPTH`,
//! `codegen::structured::MAX_STRUCTURED_DEPTH` — all 256): they descend
//! into structure *within a single word or expression*, which none of those
//! statement-tree caps bound. Each was genuinely unbounded before this fix,
//! so a single `expr {((((…))))}` word or a `[a [b [c …]]]`-nested argument
//! could drive native-stack depth arbitrarily deep and abort the process
//! with an uncatchable `SIGABRT`, rather than returning a normal result.
//!
//! Centralised here as one source of truth per category so the ~30 walkers
//! that share each cap cannot drift apart. The "what happens when the cap
//! trips" behaviour stays domain-specific at each call site (a safe
//! conservative fallback matching that function's own return contract — see
//! the per-site comments), exactly as `tcl_core_types::RecursionLimit`'s
//! module docs prescribe.
//!
//! [`MAX_SOURCE_NEST_DEPTH`] joins them for the third category — the
//! *braced-body* descent — and, unlike the two above, is not a convention
//! number at all: it is arithmetic over a stated stack budget and a measured
//! per-level cost. See its docs for why 256 was not (issue #1654).

/// Depth cap for every walk over an `[expr]` operator-tree AST
/// ([`tcl_syntax::expr::ast::ExprNode`], re-exported as
/// [`crate::ExprNode`]) — issue #996.
///
/// `ExprNode` nests via its `Binary`/`Unary`/`Ternary`/`Call` variants:
/// `expr {((((…))))}` or a long `1+1+1+…` chain places one operator node
/// per source level, and each of the ~26 independent functions that walk
/// these trees recurses once per level with no statement-tree cap in the
/// way (an expression's operator nesting is orthogonal to how deeply the
/// enclosing `Script`/`Statement` tree nests). 256 mirrors this crate's
/// established full-tree recursion convention (`MAX_BODY_DEPTH`,
/// `MAX_LOWER_NEST_DEPTH`, `MAX_OPTIMISER_WALK_DEPTH`,
/// `MAX_STRUCTURED_DEPTH`): comfortably beyond any realistic hand-written
/// expression, while far below the empirical native-stack crash threshold
/// on a 2 MiB thread (`cargo test`'s per-test default), where these walkers
/// overflow only in the low thousands of levels.
pub(crate) const MAX_EXPR_NODE_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Depth cap for every walk over nested `[cmd …]` command-substitution
/// *raw text within a single word* — issue #996.
///
/// These walkers re-scan the text inside each `[…]` substitution (or each
/// `ArgRole::Body`/`apply`-lambda body word) and recurse into any nested
/// `[…]` they find: `[a [b [c …]]]` nested N deep, or
/// `catch {catch {catch {…}}}` / `apply {{} {apply {{} {…}}}}` nested N
/// deep, drives native-stack depth N levels down. This nesting lives
/// *inside one argument word's text*, so the crate's `MAX_LOWER_NEST_DEPTH`
/// cap (which bounds recursion over *braced bodies* in the lowered IR, not
/// brackets embedded in one word) never sees it — genuinely unbounded
/// before this fix. 256 matches [`MAX_EXPR_NODE_DEPTH`] and the crate-wide
/// full-tree convention, for the same reasons.
pub(crate) const MAX_BRACKET_TEXT_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// The smallest native stack the braced-body walks below must run to
/// completion on — the platform default thread stack, 2 MiB on Linux.
///
/// It is what `std::thread::spawn`, a Tokio worker, and `cargo test`'s
/// per-test thread all hand a caller who asks for nothing in particular.
/// The `tcl` CLI, `tcl-lsp-server` and `tcl-mcp` deliberately ask for 64 MiB
/// (issue #996's `WORKER_STACK_SIZE`), so their own entry points have 32×
/// this — but a cap is a property of the walk, not of one caller, and a
/// crate this one is embedded in owes us no such courtesy. Sizing to the
/// floor keeps "the analyser aborts the process" off the table for every
/// caller instead of only the ones we ship.
pub(crate) const MIN_SOURCE_WALK_STACK: u32 = 2 * 1024 * 1024;

/// The part of [`MIN_SOURCE_WALK_STACK`] the recursive descent may **not**
/// spend: everything above it (the caller's own frames, the LSP request
/// handler, the test harness) plus the deepest non-recursive work below it
/// (segmenting one command, building its tokens).
///
/// A quarter is far more than the measurement needs — a 2 MiB thread was
/// observed to reach 112 lowering levels before aborting, i.e. 2,112,768
/// bytes of descent against a 2,097,152-byte stack, which puts the
/// non-descent overhead in the tens of kilobytes — but the reserve also
/// absorbs the difference between the deepest leaf this measurement
/// happened to reach and the deepest one some other input reaches.
const SOURCE_WALK_STACK_RESERVE: u32 = MIN_SOURCE_WALK_STACK / 4;

/// Worst per-level native-stack cost across the braced-body walk family,
/// **measured**, rounded up.
///
/// Taken with a stack-pointer probe at the depth-guarded entry of each
/// walk, on x86-64 Linux, in a `dev`-profile build — the fattest frames the
/// code ever has, and the profile `cargo test` and every developer run use:
///
/// | walk | bytes per nesting level |
/// |---|---|
/// | `lowering::Lowerer::lower_body` ↔ `lower_segmented` ↔ `lower_command` ↔ `lower_foreach` | 18,864 |
/// | `cfg_builder::CfgBuilder::lower_script` | 8,288 |
/// | `analyser::commands::Analyser::analyse_body` | 3,840 |
///
/// The lowering chain sets the number: eight Rust frames per braced-body
/// level, several of them large. 20 KiB is that 18,864 rounded up, so the
/// arithmetic below keeps a little slack even before the reserve.
pub(crate) const SOURCE_WALK_BYTES_PER_LEVEL: u32 = 20 * 1024;

/// Depth cap for the braced-body descent shared by the lowering, CFG-builder
/// and analyser walks — issue #1654.
///
/// All three recurse once per `{ … }` nesting level over the same document,
/// and all three carried a hand-picked 256 that matched this crate's
/// full-tree convention. That number was never checked against a stack: at
/// the lowering walk's measured 18,864 bytes a level, 256 levels want about
/// 4.6 MiB, so ~400 nested `foreach` bodies aborted the process on any
/// default-stack thread — the cap tripped at 256 long after the stack ran
/// out at ~112 (issue #1654; the containment the caps exist to provide,
/// absent exactly where it was claimed).
///
/// So it is derived rather than chosen: the levels
/// [`MIN_SOURCE_WALK_STACK`] pays for at
/// [`SOURCE_WALK_BYTES_PER_LEVEL`] each, after
/// [`SOURCE_WALK_STACK_RESERVE`] is set aside. Every input is one of
/// those three numbers, each of which says what it is and can be
/// re-measured; the answer falls out. `the_source_walk_cap_fits_its_stack_budget`
/// re-checks the claim by running a cap-deep document on a thread sized to
/// the budget, so frame growth in any of the three walks fails a test
/// instead of resurfacing as an abort.
///
/// The result is far below 256 and still far above anything a human writes;
/// past it each walk degrades the way it already did — the lowering emits a
/// `Statement::Barrier` for the unread region, the analyser reports E207 and
/// stops descending.
pub(crate) const MAX_SOURCE_NEST_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(
        (MIN_SOURCE_WALK_STACK - SOURCE_WALK_STACK_RESERVE) / SOURCE_WALK_BYTES_PER_LEVEL,
    );

#[cfg(test)]
mod tests {
    use super::{
        MAX_SOURCE_NEST_DEPTH, MIN_SOURCE_WALK_STACK, SOURCE_WALK_BYTES_PER_LEVEL,
        SOURCE_WALK_STACK_RESERVE,
    };

    fn nested_foreach(levels: usize) -> String {
        (0..levels)
            .map(|i| format!("foreach v{i} {{a b}} {{\n"))
            .chain(std::iter::once("set inner 1\n".to_owned()))
            .chain((0..levels).map(|_| "}\n".to_owned()))
            .collect()
    }

    /// Whether analysing `levels`-deep nesting reports having stopped.
    fn reports_over_depth(levels: usize) -> bool {
        crate::analyser::Analyser::new()
            .analyse(&nested_foreach(levels), "tcl9.0")
            .diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::E207)
    }

    /// A document nested several times deeper than the cap.
    ///
    /// Past the cap rather than exactly at it, and by a wide margin, so the
    /// test answers two questions at once: every walk on this path runs its
    /// full budget (the deepest descent the cap permits), *and* every walk
    /// on this path actually stops there. A walk that kept its own larger
    /// bound would sail past and overflow; one that merely fits would not
    /// be distinguishable from one that stops.
    fn over_cap_source() -> String {
        nested_foreach(MAX_SOURCE_NEST_DEPTH.0 as usize * 4)
    }

    #[test]
    fn the_source_walk_cap_is_the_arithmetic_it_claims_to_be() {
        assert_eq!(
            MAX_SOURCE_NEST_DEPTH.0,
            (MIN_SOURCE_WALK_STACK - SOURCE_WALK_STACK_RESERVE) / SOURCE_WALK_BYTES_PER_LEVEL,
        );
        // Deep enough that no hand-written source reaches it, and far below
        // the 256 that did not fit (issue #1654).
        assert!((32..256).contains(&MAX_SOURCE_NEST_DEPTH.0));
        // The reserve is the one input that is a *policy* rather than a
        // measurement, and the one `the_source_walk_cap_fits_its_stack_budget`
        // cannot judge for itself — that test sizes its thread from the
        // reserve, so a reserve shrunk to nothing shrinks the standard it is
        // held to as well. Pin a floor here instead: the margin exists to
        // absorb frame-cost drift between measurements and a deeper leaf
        // than the probe happened to reach, and neither is worth a rounding
        // error.
        const { assert!(SOURCE_WALK_STACK_RESERVE >= MIN_SOURCE_WALK_STACK / 8) }
    }

    /// The braced-body walks share one depth, which is the whole reason the
    /// budget can be reasoned about at all: three walks over one document,
    /// one number, so no consumer depends on one pass reaching deeper than
    /// another. Each keeps its own named constant, so nothing but a test
    /// stops one from drifting back to a private number that happens to
    /// fit — which the analyser's 3,840 bytes a level easily would.
    ///
    /// Brackets the analyser's own trip point around
    /// [`MAX_SOURCE_NEST_DEPTH`] rather than pinning it exactly: how a
    /// document's outermost script maps onto the first `body_depth` is that
    /// walk's business, and this is a statement about which *cap* it obeys.
    #[test]
    fn the_analyser_stops_at_the_shared_cap() {
        let cap = MAX_SOURCE_NEST_DEPTH.0 as usize;
        assert!(
            !reports_over_depth(cap - 2),
            "nesting below the shared cap must analyse in full"
        );
        assert!(
            reports_over_depth(cap + 2),
            "nesting above the shared cap must report that the walk stopped"
        );
    }

    /// The claim [`MAX_SOURCE_NEST_DEPTH`] makes, re-checked rather than
    /// asserted: a document nested well past the cap completes on a stack
    /// the size of the budget the cap was divided out of — so every walk
    /// this document drives both honours the shared cap and fits inside the
    /// budget that cap was derived from.
    ///
    /// Sized to `MIN_SOURCE_WALK_STACK - SOURCE_WALK_STACK_RESERVE` and not
    /// to the whole 2 MiB, so passing here proves the reserve is genuinely
    /// spare on a real default-stack thread rather than quietly spent. If
    /// any of the three walks grows fatter frames than
    /// [`SOURCE_WALK_BYTES_PER_LEVEL`] records, this aborts — loudly, in
    /// the one test whose job is to notice, instead of in a user's editor.
    ///
    /// **All three walks are driven explicitly, not only through
    /// `analyse`.** `analyse` does reach the other two today — measured, by
    /// fattening `CfgBuilder::lower_script` by 24 KiB a level and by
    /// regressing `MAX_LOWER_NEST_DEPTH` to 256, both of which abort an
    /// analyse-only version of this test. But it reaches them
    /// *conditionally*: `AnalyserState::whole_file_command_trust` lowers the
    /// document behind a `head_may_fold` gate, and the CFG arrives via
    /// `unit_scope`. Which passes a given source shape triggers is a
    /// property of those gates, not of this budget, and a change to them
    /// would silently take the coverage away while leaving the test green.
    /// Calling `lower_to_ir` and `build_cfg` here makes the coverage
    /// unconditional and states which walks are being claimed — the more so
    /// because lowering is 4.9× the analyser's per-level cost and is what
    /// sets this budget in the first place.
    #[test]
    fn the_source_walk_cap_fits_its_stack_budget() {
        let source = over_cap_source();
        let (diagnostics, blocks) = std::thread::Builder::new()
            .stack_size(
                usize::try_from(MIN_SOURCE_WALK_STACK - SOURCE_WALK_STACK_RESERVE)
                    .expect("the budget fits a usize"),
            )
            .spawn(move || {
                let diagnostics = crate::analyser::Analyser::new()
                    .analyse(&source, "tcl9.0")
                    .diagnostics
                    .len();
                let registry =
                    tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
                let module = crate::lowering::lower_to_ir(&source, registry);
                let cfg = crate::cfg_builder::build_cfg(&module, false);
                let blocks: usize = cfg.top_level.blocks.len()
                    + cfg
                        .procedures
                        .values()
                        .map(|f| f.blocks.len())
                        .sum::<usize>();
                (diagnostics, blocks)
            })
            .expect("spawn budget-sized thread")
            .join()
            .expect("every walk must return rather than abort the process");
        assert!(
            diagnostics > 0,
            "a document past the cap must say so, not fall silent"
        );
        assert!(
            blocks > 0,
            "lowering and the CFG builder must produce a truncated-but-real \
             result past the cap, not an empty one"
        );
    }
}
