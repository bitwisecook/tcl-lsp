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

use super::{
    SinkCall, SourceWarningMetadata, SourceWarningRead, TaintSourceContext, selected_sink_schema,
};
use crate::compilation_unit::CompilationUnit;
use crate::registry_invocation::AuthoredSourceTaintInvocation;
use crate::ssa::SsaSourceView;
use tcl_registry::CommandRegistry;

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
    panic!("genuine profileless original eval point");
}

#[test]
fn present_profileless_source_sink_owner_is_terminal_before_legacy_metadata() {
    let registry = CommandRegistry::build_default();
    assert!(registry.profile().is_none());
    let text = "eval $x";
    let unit = CompilationUnit::build_for(text, &registry, false);
    let point = eval_point(&unit);
    let tokens = point.source_tokens().unwrap();
    let selected = AuthoredSourceTaintInvocation::capture(tokens, &registry).unwrap();
    let effective = selected.effective().clone();
    let command = selected.command().to_owned();
    let args = effective
        .argument_spellings(&tokens.argv_texts[1..])
        .unwrap();
    let metadata = SourceWarningMetadata::Authored(Box::new(selected));
    let source = TaintSourceContext::for_unit_function(&registry, &unit, &unit.top_level);
    assert!(source.allows_nominal_metadata());
    assert!(source.metadata.is_none());
    assert!(source.module().is_some());
    let call = SinkCall {
        source,
        command: &command,
        args: &args,
        tokens: Some(tokens),
        registry: &registry,
        effective: Some(&effective),
        argument_offset: 0,
        source_read: Some(SourceWarningRead {
            metadata: &metadata,
            point,
        }),
    };
    assert!(selected_sink_schema(&call).is_some());
    let foreign = CompilationUnit::build_for(text, &registry, false);
    let wrong_point = SinkCall {
        source_read: Some(SourceWarningRead {
            metadata: &metadata,
            point: eval_point(&foreign),
        }),
        ..call
    };
    assert!(selected_sink_schema(&wrong_point).is_none());
    let mut changed = tokens.clone();
    changed.argv_texts[1] = "$donated".to_owned();
    assert_ne!(changed.argv_texts, tokens.argv_texts);
    let wrong_vector = SinkCall {
        tokens: Some(&changed),
        ..call
    };
    assert!(selected_sink_schema(&wrong_vector).is_none());
    let untagged = SinkCall {
        source_read: None,
        ..call
    };
    assert!(selected_sink_schema(&untagged).is_some());
}
