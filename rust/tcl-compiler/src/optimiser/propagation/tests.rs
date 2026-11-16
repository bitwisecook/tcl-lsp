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

use super::*;
use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present, fixture_proc};
use tcl_registry::CommandRegistry;

use crate::interprocedural::InterproceduralAnalysis;

mod command_substitution_folds;
mod constant_substitution;
mod load_forwarding;
mod proc_folds;
mod retained_substitutions;
mod return_folds;
mod store_to_load_forwarding;

fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

fn proc_fold_context(cu: &CompilationUnit, name: &str) -> String {
    let summary = cu.interproc.as_ref().and_then(|ia| fixture_proc(ia, name));
    let summary = summary.map(|s| {
        (
            s.pure,
            s.can_fold_static_calls,
            s.has_barrier,
            s.has_unknown_calls,
            s.writes_global,
            &s.constant_return,
        )
    });
    let body = cu.function(name).map(|fu| {
        let statements: Vec<_> = fu
            .cfg
            .blocks
            .iter()
            .map(|(id, block)| {
                let kinds: Vec<_> = block
                    .statements
                    .iter()
                    .map(std::mem::discriminant)
                    .collect();
                (
                    id,
                    kinds,
                    block.terminator.as_ref().map(std::mem::discriminant),
                )
            })
            .collect();
        (statements, &fu.sccp.values, &fu.sccp.executable_blocks)
    });
    format!("summary(pure,fold,barrier,unknown,global,return)={summary:?}, body={body:?}")
}

fn run_pass(source: &str) -> Vec<Optimisation> {
    let reg = registry();
    let cu = CompilationUnit::build_for(source, &reg, false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    // Mirror `optimise_unit_raw`'s production wiring — `run_load_forwarding`
    // (O102) needs `ctx.registry` for its intervening-call purity scan, and
    // the trace/alias safety gates need `ctx.ir_module`. Without these two
    // fields set, a hand-built `PassContext` silently loses O102 coverage
    // (and O127's pre-existing trace-purity gate), which is exactly the bug
    // this test helper would otherwise mask.
    ctx.registry = Some(&reg);
    ctx.ir_module = Some(&cu.ir_module);
    run(&mut ctx, &cu);
    ctx.optimisations
}

fn fold(src: &str) -> Vec<String> {
    crate::optimiser::optimise_raw(src, &registry(), None)
        .into_iter()
        .filter(|o| o.code == DiagCode::O129)
        .map(|o| o.replacement)
        .collect()
}

#[test]
fn run_passes_dispatches_propagation() {
    let reg = registry();
    let cu = CompilationUnit::build_for("set x 9\nputs $x", &reg, false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    ctx.registry = Some(&reg);
    super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::Propagation]);
    assert!(
        ctx.optimisations.iter().any(|o| o.code == DiagCode::O100),
        "expected O100 via run_passes, got {:?}",
        ctx.optimisations,
    );
}
