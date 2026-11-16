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

use crate::registry_invocation::source_structure::{
    OriginalRegistrySource, OriginalSourceRegistryContext,
};

/// Whether the clause body's `set y 1` inherits an unknown earlier mutation.
fn body_inherits_unknown_mutation(source: &str) -> bool {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let syntax = OriginalSourceRegistryContext::capture(source, input.clone());
    let body = "set y 1";
    let start = source.find(body).unwrap();
    let children = crate::segmenter::segment_commands_with_offset_and_config(
        &source[start..start + body.len()],
        u32::try_from(start).unwrap(),
        input.lexer_config().nested(),
    );
    let [child] = children.as_slice() else {
        panic!("one original body command: {source}")
    };
    let words = syntax.words(child).expect(source);
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        panic!("original transition advice: {source}")
    };
    advice
        .obligations()
        .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
}

#[test]
fn selected_condition_children_leave_no_parent_debt_unless_unwalked_phases_remain() {
    // Effect-free children of a selected first-match condition run once,
    // before the body, and are threaded rather than marked.
    assert!(!body_inherits_unknown_mutation(
        "if {[package vsatisfies 8.6 8.5]} {set y 1}"
    ));
    // A math function may be a user `tcl::mathfunc` command the walk does
    // not enter, a variable read may fire a trace, and a loop condition
    // runs again after its body: each keeps the parent operation uncertain.
    for source in [
        "if {[package vsatisfies 8.6 8.5] && abs(1)} {set y 1}",
        "if {[package vsatisfies 8.6 8.5] && $x} {set y 1}",
        "while {[package vsatisfies 8.6 8.5]} {set y 1}",
    ] {
        assert!(body_inherits_unknown_mutation(source), "{source}");
    }
}
