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
/// ([`tcl_syntax::expr::ast::ExprNode`]).
///
/// Binary, unary, ternary and function-call nodes can nest independently of
/// the enclosing script's braced-body depth. The 256-node cap bounds these
/// recursive walks separately from [`MAX_SOURCE_NEST_DEPTH`].
#[doc(hidden)]
pub const MAX_EXPR_NODE_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// Depth cap for recursive descent into command substitutions inside a word.
///
/// Nested `[a [b [c ...]]]` substitutions occupy one argument word, so a
/// braced-body limit cannot bound this axis. Consumers share the 256-level
/// bracket-text limit and retain their own explicit refusal at its boundary.
pub const MAX_BRACKET_TEXT_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Native-stack budget for the shared braced-body recursion limit: 2 MiB.
///
/// The limit applies to all callers, including ordinary worker and test
/// threads. Executable entry points with larger stacks use the same limit.
pub const MIN_SOURCE_WALK_STACK: u32 = 2 * 1024 * 1024;

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
pub const MAX_SOURCE_NEST_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(
    (MIN_SOURCE_WALK_STACK - SOURCE_WALK_STACK_RESERVE) / SOURCE_WALK_BYTES_PER_LEVEL,
);

#[cfg(test)]
mod tests {
    use super::*;
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
}
