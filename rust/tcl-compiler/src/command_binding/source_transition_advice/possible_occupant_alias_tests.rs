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
fn caller_namespace_alias_refused_past_a_possible_occupant_keeps_the_registry_barrier() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let policy = AdviceNamingPolicy::Logical(input.clone());
    let source = "run helper";
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        input.lexer_config(),
    )
    .unwrap();
    let written = original_words(&plan.commands[0].words, &policy).unwrap();
    let run = written[0].input.clone().unwrap();
    let helper = written[1].input.clone().unwrap();
    let mut graph = AdviceGraph::new(
        &context,
        &policy,
        InvocationDialect::of_profile(input.unit_profile()),
    )
    .unwrap();
    let namespace = ByteNamespacePath::root().with_child("A");
    graph.logical_namespace = Some(namespace.clone());
    let slot =
        |bytes: &[u8]| tcl_syntax::naming::authored_source_command_slot(&namespace, bytes).unwrap();
    // `::A::run` is a caller-namespace alias to `helper`; `::helper` is a
    // user command, which the raw target lookup reaches and blocks on.
    graph.cells.insert(
        slot(b"run"),
        AdviceCell::Alias {
            target: Box::new(helper),
            prefix: Vec::new(),
            lookup: AliasTargetLookup::CallerNamespace,
            lineage: Vec::new(),
        },
    );
    graph.cells.insert(
        AdviceNamingPolicy::logical_slot(b"::helper").unwrap(),
        AdviceCell::Shadowed,
    );
    assert!(graph.blocks_registry_source(&run));
    // A possible `::A::helper` refuses the target selection; the barrier
    // the raw target recorded must remain.
    graph.possible_occupants.insert(slot(b"helper"));
    assert!(graph.resolve(&run).is_none());
    assert!(graph.blocks_registry_source(&run));
}
