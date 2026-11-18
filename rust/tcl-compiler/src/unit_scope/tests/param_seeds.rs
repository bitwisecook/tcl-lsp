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

//! Call-site evidence per parameter position: omitted and defaulted arguments, and the retained metadata owner.

use super::*;

/// A call that omits a defaulted parameter binds it to its **default**, an
/// unknown value — so a literal at another call site cannot claim the
/// position. `binds_position` is what encodes that.
#[test]
fn an_omitted_argument_stops_a_position_being_uniform() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {a {b x}} { return $a }\nhelper 1 two\nhelper 1\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    let helper = evidence.get("::helper").expect("calls recorded");
    assert_eq!(helper.uniform_literal_at(0), Some("1"));
    assert_eq!(helper.arg_counts, [1, 2].into_iter().collect());
    assert!(!helper.binds_position(1));
    assert_eq!(helper.uniform_literal_at(1), None);
}

#[test]
fn an_omitted_defaulted_argument_poisons_its_slot() {
    let ev = evidence("proc helper {a {b DEFAULT}} { return $a }\nhelper one\n");
    assert_eq!(slot(&ev, "::helper", 0), (vec!["one".into()], false));
    assert_eq!(
        uniform(&ev, "::helper", 1),
        None,
        "the omitted argument leaves b bound to its default",
    );
}

fn supplied_unit_callers(
    source: &str,
    module: &IrModule,
    input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> CallSiteEvidence {
    let registry = module.resolved_registry();
    let cfg = crate::cfg_builder::build_cfg(module, false);
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    );
    let identities = input.map_or_else(
        || identities.clone(),
        |input| {
            identities
                .clone()
                .with_resolved_analysis_input(input.clone())
        },
    );
    collect_call_site_constants(
        &cfg,
        &[],
        &module.procedures,
        &module.future_call_sites,
        CallSiteSourceContext {
            registry,
            module,
            declared: None,
            input,
            identities: &identities,
        },
    )
}

#[test]
fn unit_caller_seeds_require_the_retained_metadata_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual source-owned caller operands, not a complete Native command world.
    let context = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let source = "proc helper {mode} {return $mode}; helper FIXED";
    let module = supplied_unit_fixture(source, &context);
    let input = module.source_metadata_input.as_ref().unwrap();
    let current = supplied_unit_callers(source, &module, Some(input));
    assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("FIXED"));
    let foreign = supplied_unit_input(
        input,
        crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        ),
    );
    for withheld in [Some(&foreign), None] {
        let evidence = supplied_unit_callers(source, &module, withheld);
        assert!(evidence.get("::helper").unwrap().opaque_caller);
        assert_eq!(uniform(&evidence, "::helper", 0), None);
    }
}
