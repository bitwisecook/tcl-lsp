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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use tcl_dialect::model::{Family, SurfaceQuery};

mod function_level;
mod loop_invariants;
mod partial_redundancies;
mod redundancies;
mod stable_call_reuse;
mod statement_occurrences;
mod value_table;

/// The default document's `${…}` close rule — these unit tests build no
/// dialect profile, so they exercise the same style the lexer would use.
const STYLE: tcl_dialect::BracedVarStyle = tcl_dialect::BracedVarStyle::Tcl9Nesting;

use super::*;
use crate::semantic_analysis::ExecutableAnalysisAvailability;

/// The catalogue a document under `environment` is analysed against: the
/// shipped registry projected for that release's analyser profile.
fn release_registry(environment: &str) -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment(environment).analyser_profile(),
    )
}

fn entry(key: &[&str], block: &str, idx: usize) -> ValueEntry {
    let key_owned: ExprKey = key.iter().map(|s| (*s).into()).collect();
    ValueEntry {
        key: key_owned.clone(),
        block: block.into(),
        statement_index: idx,
        span: Span::new(0, 0),
        expression_text: key_owned.join(" "),
    }
}

// Canonicalisation + messages.

/// A bare SSA shell (no blocks) for tests that only need the variable
/// interner to resolve canonicalisation names.
fn bare_ssa() -> SsaFunction {
    SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()])
}

// find_redundancies driver.

use crate::cfg::{Block, Function, Terminator};
use crate::ssa::{SsaBlock, SsaStatement};
use std::collections::HashMap as Map;

/// The [`BlockId`] a block name was interned to in `cfg`.
fn id_of(cfg: &Function, name: &str) -> BlockId {
    cfg.block_id(name).expect("interned")
}

fn llength_call() -> Statement {
    llength_call_at(0, 0)
}

fn llength_call_at(start: u32, end: u32) -> Statement {
    Statement::Call {
        span: Span::new(start, end),
        command: "llength".into(),
        canonical_command: None,
        args: vec!["$x".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

fn ssa_stmt_for(ssa: &mut SsaFunction, stmt: Statement, uses_x_ver: Option<u32>) -> SsaStatement {
    let mut uses = Map::new();
    if let Some(v) = uses_x_ver {
        uses.insert(ssa.intern_var("x"), v);
    }
    SsaStatement {
        statement: stmt,
        uses,
        defs: Map::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

fn empty_ssa_block(name: &str) -> SsaBlock {
    SsaBlock {
        name: name.into(),
        phis: Vec::new(),
        statements: Vec::new(),
        entry_versions: Map::new(),
        exit_versions: Map::new(),
    }
}

/// Every block of a hand-built CFG, as the executable set. These tests
/// exercise the loop / availability machinery on CFGs with no SCCP run,
/// so they hand it the whole CFG, stating the premise at the call site
/// instead of deriving it inside the pass.
fn all_blocks(cfg: &Function) -> std::collections::HashSet<BlockId> {
    cfg.blocks.keys().copied().collect()
}
