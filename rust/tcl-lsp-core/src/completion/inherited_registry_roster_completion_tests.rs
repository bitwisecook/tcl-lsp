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
use tcl_registry::spec::ObjectClassSpec;
use tcl_registry::{CommandSpec, SubCommand};

#[test]
fn inherited_completion_cannot_borrow_a_surface_only_provider_excluded_by_actual_roster() {
    static METHODS: [SubCommand; 1] = [SubCommand {
        name: "rosterLeak",
        ..SubCommand::DEFAULT
    }];
    static CHILD: ObjectClassSpec = ObjectClassSpec {
        class_name: "RosterChild",
        instance_methods: &[],
        superclasses: &["coroutine"],
        allow_unknown_methods: false,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Strict,
    };
    static PARENT: ObjectClassSpec = ObjectClassSpec {
        class_name: "coroutine",
        instance_methods: &METHODS,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Strict,
    };
    tcl_spectcl::core_surfaces::ensure();
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "RosterChild",
        object_class: Some(&CHILD),
        ..CommandSpec::DEFAULT
    });
    registry.insert(CommandSpec {
        name: "coroutine",
        object_class: Some(&PARENT),
        surface: Some(SpecSurface::TCL86),
        ..CommandSpec::DEFAULT
    });
    // Jim 0.79 has no Native name recipe. Its actual inherited Tcl roster
    // independently excludes coroutine despite the inherited surface edge.
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let analysis = crate::source_structure::analyse_document(" ", profile, &registry, config);
    assert!(analysis.allows_lexical_declaration_advice());
    let floor = crate::document_floor::DocumentFloor::new(&analysis, profile);
    let context = floor.context().unwrap();
    let surface_owner = registry
        .get_for_surface("coroutine", Some(context.authoring_query()))
        .unwrap();
    assert!(std::ptr::eq(
        surface_owner.object_class.unwrap(),
        &raw const PARENT
    ));
    assert!(context.spec_available(surface_owner));
    assert!(context.resolve_spec(&registry, "coroutine").is_none());
    assert!(registry_method_items(&analysis, &registry, "RosterChild", profile).is_none());
}
