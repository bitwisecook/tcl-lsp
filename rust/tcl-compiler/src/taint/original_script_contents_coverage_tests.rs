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

use super::{SourceWarningMetadata, SourceWarningRead, TaintLattice};
use crate::compilation_unit::CompilationUnit;
use crate::registry_invocation::AuthoredSourceTaintInvocation;
use crate::ssa::SsaSourceView;

fn eval_point(unit: &CompilationUnit) -> SsaSourceView<'_> {
    for (&block, body) in &unit.top_level.ssa.blocks {
        for index in 0..body.statements.len() {
            let point = SsaSourceView::at_statement(&unit.top_level.ssa, block, index);
            if point
                .source_tokens()
                .and_then(|tokens| tokens.argv_texts.first())
                .is_some_and(|head| head == "eval")
            {
                return point;
            }
        }
    }
    panic!("genuine original eval point");
}

fn set_location(unit: &CompilationUnit, rhs: &str) -> (crate::cfg::BlockId, usize) {
    for (&block, body) in &unit.top_level.ssa.blocks {
        for index in 0..body.statements.len() {
            let point = SsaSourceView::at_statement(&unit.top_level.ssa, block, index);
            if point.source_tokens().is_some_and(|tokens| {
                tokens.argv_texts.len() == 3
                    && tokens.argv_texts[0] == "set"
                    && tokens.argv_texts[1] == "x"
                    && tokens.argv_texts[2] == rhs
            }) {
                return (block, index);
            }
        }
    }
    panic!("actual original Set site retained independently of IR operation kind");
}

#[test]
fn root_source_versions_stay_unknown_and_only_complete_literal_coverage_is_clean() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (text, literal) in [
        ("set x SAFE; eval $x", true),
        ("set x SAFE; set z OTHER; eval $x", true),
        ("set x SAFE; set x $y; eval $x", false),
        ("set x SAFE; set x $a(k); eval $x", false),
        ("set x SAFE; unknown_writer; eval $x", false),
        ("set x SAFE; upvar #0 y x; eval $x", false),
        ("set x SAFE; if {$condition} {set x $y}; eval $x", false),
        ("set x SAFE; eval $x [set x $y]", false),
    ] {
        let unit = CompilationUnit::build_for(text, registry, false);
        let point = eval_point(&unit);
        let tokens = point.source_tokens().unwrap();
        let selected = AuthoredSourceTaintInvocation::capture(tokens, registry)
            .expect("actual conditional source sink owner");
        let metadata = SourceWarningMetadata::Authored(Box::new(selected));
        let read = SourceWarningRead {
            metadata: &metadata,
            point,
        };
        let value = read
            .taint("x", &tokens.words()[1], &unit.top_level.taints)
            .unwrap();
        assert_eq!(value.is_tainted(), !literal, "{text}");
        let roots = unit
            .top_level
            .ssa
            .original_script_source_symbols()
            .collect::<Vec<_>>();
        assert_ne!(roots, [] as [crate::ssa::Symbol; 0]);
        for ((symbol, _), value) in unit.top_level.taints.iter() {
            if roots.contains(symbol) {
                assert_eq!(*value, TaintLattice::tainted(), "{text}");
            }
        }
        for symbol in roots {
            assert_eq!(
                unit.top_level.taints.get(&(symbol, 0)),
                Some(&TaintLattice::tainted())
            );
            assert!(unit.top_level.ssa.cell_key(symbol).is_none());
        }
    }
}

#[test]
fn removing_an_unknown_middle_statement_cannot_create_literal_source_coverage() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = CompilationUnit::build_for("set x SAFE; set x $y; eval $x", registry, false);
    let point = eval_point(&unit);
    let tokens = point.source_tokens().unwrap();
    let selected = AuthoredSourceTaintInvocation::capture(tokens, registry).unwrap();
    let metadata = SourceWarningMetadata::Authored(Box::new(selected));
    let mut altered = unit.clone();
    let (first_block, first_index) = set_location(&unit, "SAFE");
    let first = unit.top_level.ssa.blocks[&first_block].statements[first_index]
        .statement
        .clone();
    let (middle_block, middle_index) = set_location(&altered, "$y");
    let middle = &mut altered
        .top_level
        .ssa
        .blocks
        .get_mut(&middle_block)
        .unwrap()
        .statements[middle_index];
    assert_ne!(middle.statement, first);
    middle.statement = first;
    let read = SourceWarningRead {
        metadata: &metadata,
        point: eval_point(&altered),
    };
    assert!(
        read.taint("x", &tokens.words()[1], unit.top_level.taints.as_ref())
            .is_none()
    );
}
