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

use super::{SourceWarningMetadata, SourceWarningRead, TaintLattice, ValueKey};
use crate::compilation_unit::CompilationUnit;
use crate::registry_invocation::AuthoredSourceTaintInvocation;
use crate::ssa::SsaSourceView;
use std::collections::HashMap;

fn source_point<'a>(unit: &'a CompilationUnit, head: &str) -> SsaSourceView<'a> {
    for (&block, body) in &unit.top_level.ssa.blocks {
        for index in 0..body.statements.len() {
            let point = SsaSourceView::at_statement(&unit.top_level.ssa, block, index);
            if point
                .source_tokens()
                .and_then(|tokens| tokens.argv_texts.first())
                .is_some_and(|held| held == head)
            {
                return point;
            }
        }
    }
    panic!("genuine original source point is retained");
}

#[test]
fn unknown_source_read_keeps_requested_operand_and_point_ownership() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for text in [
        "eval $x [unknown_callback]",
        "trace add variable x read callback\neval $x",
    ] {
        let unit = CompilationUnit::build_standalone_for(text, registry, false);
        let point = source_point(&unit, "eval");
        let tokens = point.source_tokens().unwrap();
        let selected = AuthoredSourceTaintInvocation::capture(tokens, registry)
            .expect("genuine conditional sink exists even with nonquiet source context");
        assert!(selected.matches_point(point));
        assert!(!selected.has_closed_scalar_read_context(), "{text}");
        let original = &tokens.words()[1];
        let metadata = SourceWarningMetadata::Authored(Box::new(selected));
        let read = SourceWarningRead {
            metadata: &metadata,
            point,
        };
        let taints = HashMap::<ValueKey, TaintLattice>::new();
        assert!(read.taint("x", original, &taints).unwrap().is_tainted());
        assert!(read.taint("donated", original, &taints).is_none());
        let foreign = CompilationUnit::build_standalone_for(text, registry, false);
        let foreign_read = SourceWarningRead {
            metadata: &metadata,
            point: source_point(&foreign, "eval"),
        };
        assert!(foreign_read.taint("x", original, &taints).is_none());
        let other = CompilationUnit::build_standalone_for("eval $y", registry, false);
        let other_point = source_point(&other, "eval");
        let unowned = &other_point.source_tokens().unwrap().words()[1];
        assert!(read.taint("y", unowned, &taints).is_none());
        assert!(!read.references("donated", original, tcl_lexer::LexerConfig::default()));
    }
}
