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

#[test]
fn source_syntax_requires_every_actual_alternative_and_full_input() {
    let source = "proc f {x} {if {$x <= 1} {return 1}}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = analysis.resolved_registry().unwrap();
    let bindings = analysis
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref();
    let offset = source.find("if {").unwrap();
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        &source[offset..],
        u32::try_from(offset).unwrap(),
        input.lexer_config(),
    )
    .remove(0);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        input.lexer_config(),
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let views = bindings
        .original_compiler_syntax_alternatives(&tokens, input, registry)
        .unwrap();
    assert!(!views.is_empty());
    assert!(views.iter().all(
        |view| view.source_lookup_advice(&tokens).is_some_and(|advice| {
            advice.closed_authored_simulation_source_lookup(&tokens, view.metadata(), registry)
        })
    ));
    let site = views[0].site().clone();
    let mut withdrawn = bindings.clone();
    let visits = withdrawn.compiler_invocations.get_mut(&site).unwrap();
    let mut missing = visits[0].clone();
    missing.policy = None;
    visits.push(missing);
    assert!(
        withdrawn
            .original_compiler_syntax_alternatives(&tokens, input, registry)
            .is_none()
    );
    let mut changed = bindings.clone();
    let visits = changed.compiler_invocations.get_mut(&site).unwrap();
    let mut opaque = visits[0].clone();
    Arc::make_mut(&mut opaque.table)
        .state
        .opaque_binding_mutation = true;
    visits.push(opaque);
    let all = changed
        .original_compiler_syntax_alternatives(&tokens, input, registry)
        .unwrap();
    assert!(all.len() > views.len());
    assert!(all.iter().any(
        |view| !view.source_lookup_advice(&tokens).is_some_and(|advice| {
            advice.closed_authored_simulation_source_lookup(&tokens, view.metadata(), registry)
        })
    ));
    let other = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        other
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref()
            .original_compiler_syntax_alternatives(&tokens, input, registry)
            .is_none()
    );
}
