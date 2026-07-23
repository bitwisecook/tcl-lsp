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
