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

use super::test_body_identity::{edge_labels, fixture_id, fixture_id_if_present, fixture_proc};

mod body_scan;
mod call_graph;
mod call_targets;
mod callback_edges;
mod method_summaries;
mod parameter_observations;
mod return_summaries;
mod scope_aliases;
mod summary_fixpoints;
mod summary_identity;

fn fixture_method<'a>(
    analysis: &'a InterproceduralAnalysis,
    label: &str,
) -> Option<&'a MethodSummary> {
    let mut matches = analysis
        .methods
        .values()
        .filter(|method| method.base.qualified_name == label);
    let first = matches.next()?;
    assert!(
        matches.next().is_none(),
        "fixture method label must be unambiguous"
    );
    Some(first)
}

use super::*;

#[test]
fn arity_from_names_handles_trailing_args() {
    assert_eq!(
        arity_from_names(&["a".to_owned(), "args".to_owned()]),
        Arity::at_least(1)
    );
    assert_eq!(
        arity_from_names(&["a".to_owned(), "b".to_owned()]),
        Arity::exact(2)
    );
    assert_eq!(arity_from_names(&[]), Arity::exact(0));
}

#[test]
fn proc_summary_unknown_is_conservative() {
    let s = ProcSummary::unknown("::mystery");
    assert!(s.has_unknown_calls);
    assert!(s.writes_global);
    assert!(!s.pure);
    assert_eq!(s.effect_reads, EffectRegion::UNKNOWN_STATE);
    assert_eq!(s.effect_writes, EffectRegion::UNKNOWN_STATE);
}

// summary-building tests

use crate::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

fn build(source: &str) -> InterproceduralAnalysis {
    // The fixture's identity surface below is explicitly C Tcl 8.6.
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let cu = CompilationUnit::build_for_profile(source, &registry, false, profile);
    let identities = crate::realm::document_realm_bindings_for_module(&cu.ir_module, &registry)
        .expect("fixture's retained Module source owner");
    build_interprocedural_analysis(
        &cu.ir_module,
        &registry,
        Some(profile),
        ObjectTypeMap::none(),
        &identities,
        None,
    )
}

fn original_reference_unit(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    CompilationUnit,
) {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    // These fixtures pin explicit Standalone authoring (the retained
    // `original_authored_declaration_body_units`). A profiled default
    // build is a fresh-interpreter document (user decision
    // 2026-10-19T14:20Z); Standalone authoring is the dialect-blind
    // build, still profiled through its Registry and grammar.
    let unit = CompilationUnit::build_with_options(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            dialect: None,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
    );
    (context, unit)
}
