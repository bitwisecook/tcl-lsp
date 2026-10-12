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

//! Shared native-stack depth limits for expression nodes, nested command
//! substitutions and braced script bodies.
//!
//! These are independent structural axes. Every consumer of an axis uses its
//! shared limit; exceeding it retains that consumer's explicit unknown or
//! unrepresented-result contract. [`MAX_SOURCE_NEST_DEPTH`] derives the script
//! body limit from a stack budget, reserve and measured per-level envelope.

/// Depth cap for walks over the expression operator AST
/// ([`tcl_syntax::expr::ast::ExprNode`], re-exported as [`crate::ExprNode`]).
///
/// Binary, unary, ternary and function-call nodes can nest independently of
/// the enclosing script's braced-body depth. The 256-node cap bounds these
/// recursive walks separately from [`MAX_SOURCE_NEST_DEPTH`].
pub(crate) const MAX_EXPR_NODE_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Depth cap for recursive descent into command substitutions inside a word.
///
/// Nested `[a [b [c ...]]]` substitutions occupy one argument word, so a
/// braced-body limit cannot bound this axis. Consumers share the 256-level
/// bracket-text limit and retain their own explicit refusal at its boundary.
pub(crate) const MAX_BRACKET_TEXT_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Native-stack budget for the shared braced-body recursion limit: 2 MiB.
///
/// The limit applies to all callers, including ordinary worker and test
/// threads. Executable entry points with larger stacks use the same limit.
pub(crate) const MIN_SOURCE_WALK_STACK: u32 = 2 * 1024 * 1024;

/// Stack reserved for caller frames and nonrecursive leaf work, including
/// request handlers, command segmentation and token construction.
///
/// One quarter of [`MIN_SOURCE_WALK_STACK`] is excluded from recursive descent.
const SOURCE_WALK_STACK_RESERVE: u32 = MIN_SOURCE_WALK_STACK / 4;

/// Worst per-level native-stack cost across the braced-body walk family,
/// **measured**, rounded up.
///
/// Measured from native entry-frame allocations along each recursive
/// call chain, on x86-64 Linux, in a `dev`-profile build — the fattest frames the
/// code ever has, and the profile `cargo test` and every developer run use:
///
/// | walk | bytes per nesting level |
/// |---|---|
/// | `lowering::Lowerer::lower_body` ↔ `lower_segmented` ↔ `lower_command` ↔ structured hook ↔ `lower_foreach` | about 19 KiB |
/// | `cfg_builder::CfgBuilder::lower_script` ↔ `lower_foreach` | 10,256 |
/// | `analyser::commands::Analyser::analyse_body` | 3,840 |
/// | `command_binding::SourceCommandBindings` source/command/native/conditional/body chain | 17,952 conditional / 19,440 loop |
///
/// Structured lowering and source binding retain several frames per body
/// level. Full registry invocation materialisation, selected-frame construction,
/// chunk preflight, dispatch carriers and continuation snapshots belong in
/// nonrecursive leaf helpers. Only their heap-produced selected contracts
/// remain live during descent; boxing a result alone does not remove its
/// constructor's recursive stack frame.
///
/// The 24 KiB envelope covers the measured recursive chains. Stack-budget
/// controls exercise both proved typed bodies and over-limit input.
pub(crate) const SOURCE_WALK_BYTES_PER_LEVEL: u32 = 24 * 1024;

/// Depth cap for braced-body descent shared by source binding, lowering,
/// CFG construction and analyser walks.
///
/// The cap is the available [`MIN_SOURCE_WALK_STACK`] after
/// [`SOURCE_WALK_STACK_RESERVE`], divided by [`SOURCE_WALK_BYTES_PER_LEVEL`].
/// `the_source_walk_cap_fits_its_stack_budget` runs a cap-deep document on a
/// thread with this budget, checking the shared recursion envelope.
///
/// Source interpretation operates at whole-document and isolated-script
/// ingress. Its descriptor and continuation snapshots are constructed by
/// nonrecursive leaf helpers so their constructor frames do not accumulate.
/// Beyond the cap, lowering retains an unrepresented `Statement::Barrier`
/// and the analyser reports E207 without descending into the unread region.
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

    fn nested_computed_if(levels: usize) -> String {
        (0..levels)
            .map(|_| "if [set selected] {\n")
            .chain(std::iter::once("set inner 1\n"))
            .chain((0..levels).map(|_| "}\n"))
            .collect()
    }

    #[test]
    fn proved_structured_bodies_fit_the_shared_stack_budget() {
        let levels = MAX_SOURCE_NEST_DEPTH.0 as usize - 2;
        let source = nested_foreach(levels);
        let lowered_levels = std::thread::Builder::new()
            .stack_size(
                usize::try_from(MIN_SOURCE_WALK_STACK - SOURCE_WALK_STACK_RESERVE)
                    .expect("stack budget fits usize"),
            )
            .spawn(move || {
                let registry =
                    tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
                let module = crate::lowering::lower_to_ir_with_dialect(
                    &source,
                    registry,
                    tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
                    Some(tcl_dialect::DialectProfile::find("tcl9.0").expect("profile")),
                );
                // Over-cap source proof widens and can suppress typed lowering
                // early. A below-cap input must actually retain every recursive
                // structured body, not only return a shallow opaque barrier.
                let mut script = &module.top_level;
                let mut count = 0;
                while let Some(crate::ir::Statement::Foreach { body, .. }) =
                    script.statements.first()
                {
                    count += 1;
                    script = body;
                }
                count
            })
            .expect("spawn budget-sized thread")
            .join()
            .expect("proved structured descent must fit the shared budget");
        assert_eq!(lowered_levels, levels);
    }

    #[test]
    fn source_bindings_with_computed_conditions_fit_the_shared_stack_budget() {
        let source = format!(
            "set selected 1\n{}",
            nested_computed_if(MAX_SOURCE_NEST_DEPTH.0 as usize * 4)
        );
        let statements = std::thread::Builder::new()
            .stack_size(
                usize::try_from(MIN_SOURCE_WALK_STACK - SOURCE_WALK_STACK_RESERVE)
                    .expect("stack budget fits usize"),
            )
            .spawn(move || {
                let registry =
                    tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
                let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
                    &source,
                    tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
                    registry,
                    crate::command_binding::SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                            tcl_dialect::TclVersion::V9_0,
                        )),
                        ..crate::command_binding::SourceAnalysisOptions::default()
                    },
                );
                drop(bindings);
                crate::lowering::lower_to_ir_with_dialect(
                    &source,
                    registry,
                    tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
                    Some(tcl_dialect::DialectProfile::find("tcl9.0").expect("profile")),
                )
                .top_level
                .statements
                .len()
            })
            .expect("spawn budget-sized thread")
            .join()
            .expect("computed conditions must return rather than overflow");
        assert!(
            statements > 0,
            "over-budget input retains a real outer script"
        );
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
        // the 256 that does not fit.
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
                let binding_registry = tcl_registry::CommandRegistry::build_default();
                let _bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
                    &source,
                    tcl_lexer::LexerConfig::default(),
                    &binding_registry,
                    crate::command_binding::SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                            tcl_dialect::TclVersion::V9_0,
                        )),
                        ..crate::command_binding::SourceAnalysisOptions::default()
                    },
                );
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
