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
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
use tcl_registry::definer::{DefinerFamily, DefinitionBodyGrammar, ITCL_GRAMMAR};

const OTHER_FAMILY: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Snit,
    profile: tcl_registry::definer::SNIT_PROFILE,
    ..ITCL_GRAMMAR
};

fn analysis(grammar: &'static DefinitionBodyGrammar) -> tcl_compiler::analyser::AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = CommandRegistry::build_default();
    let mut spec = registry.get("itcl::class").unwrap().clone();
    spec.name = "source_class";
    spec.required_package = None;
    spec.definition_body = Some(grammar);
    registry.insert(spec);
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::new(
            tcl_registry::model::ingress::context_for_profile(profile)
                .with_command_store(Arc::new(registry)),
        ),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse("source_class C {proc make {} {return OK}}", profile.name)
}

#[test]
fn actual_cross_document_classmethod_receivers_require_known_non_itcl_metadata() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let itcl = analysis(&ITCL_GRAMMAR);
    let other = analysis(&OTHER_FAMILY);
    let nominal = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let itcl_index =
        core_workspace_index::WorkspaceIndex::from_documents([("file:///itcl.tcl", &itcl)]);
    let other_index =
        core_workspace_index::WorkspaceIndex::from_documents([("file:///other.tcl", &other)]);
    let itcl_classes = itcl_index.classes_named("::C");
    let other_classes = other_index.classes_named("::C");
    assert_eq!(
        Backend::classmethod_dispatch_names(&itcl_classes, &[], "make", true, nominal),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        Backend::classmethod_dispatch_names(&other_classes, &[], "make", true, nominal),
        vec!["::C".to_owned()]
    );
    assert_eq!(
        Backend::classmethod_dispatch_names(&other_classes, &[], "make", false, nominal),
        [] as [std::string::String; 0]
    );
    let mut missing = other;
    missing.resolved_input = None;
    let unknown_index =
        core_workspace_index::WorkspaceIndex::from_documents([("file:///unknown.tcl", &missing)]);
    let unknown = unknown_index.classes_named("::C");
    assert_eq!(unknown[0].definer_family(), None);
    assert_eq!(
        Backend::classmethod_dispatch_names(&unknown, &[], "make", true, nominal),
        [] as [std::string::String; 0]
    );
}
