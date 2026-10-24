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

//! **Design Q6, end to end**: the compiled-in jim surface roster reaches
//! a real `jim` registry generation, and what a `jim` document is offered
//! stops being "everything Tcl 8.6 has".
//!
//! The unit tests beside the pieces cover the parser, the conversion and
//! the model's admit rule in isolation. This file is the one that would
//! have caught the over-admission: it asks the same question a completion
//! request asks — *is this head offered here?* — of the assembled
//! generation.

use tcl_registry::model::resolve_environment;

/// Every name in this list is in a stock `tclsh8.6`'s `info commands` and
/// in **no** `jimsh` built from any tag on the 0.76–0.84 ladder. Before
/// Q6 the inherited-surface edge offered a `jim` document all seventeen.
const NOT_IN_JIM: &[&str] = &[
    "auto_execok",
    "auto_import",
    "auto_load",
    "auto_load_index",
    "auto_qualify",
    "case",
    "chan",
    "coroutine",
    "encoding",
    "fblocked",
    "fcopy",
    "tclLog",
    "trace",
    "unknown",
    "unload",
    "yield",
    "yieldto",
];

/// A representative slice of the roster: heads `jimsh` really has, spread
/// across the core table, the extensions and the `stdlib`/`tclcompat`
/// procs, so a regression that dropped one source of names would show.
const IN_JIM: &[&str] = &[
    "append",
    "array",
    "catch",
    "clock",
    "dict",
    "exec",
    "file",
    "foreach",
    "format",
    "if",
    "info",
    "lassign",
    "lmap",
    "lsort",
    "namespace",
    "package",
    "proc",
    "regexp",
    "set",
    "string",
    "subst",
    "switch",
    "tailcall",
    "throw",
    "try",
    "uplevel",
    "upvar",
    "while",
];

fn jim_offers(name: &str) -> bool {
    tcl_spectcl::core_surfaces::ensure();
    resolve_environment("jim")
        .default_context_registry()
        .resolve_command(name)
        .is_some()
}

fn tcl_offers(name: &str) -> bool {
    tcl_spectcl::core_surfaces::ensure();
    resolve_environment("tcl8.6")
        .default_context_registry()
        .resolve_command(name)
        .is_some()
}

/// The defect Q6 exists to fix.
#[test]
fn a_jim_document_is_not_offered_what_jim_does_not_have() {
    for name in NOT_IN_JIM {
        assert!(
            !jim_offers(name),
            "`{name}` is in tclsh8.6 and in no jimsh, but a jim document is still offered it"
        );
    }
}

/// …without taking the surface with it. A roster that narrowed too far
/// would pass the test above and break every jim document, so the two
/// halves are asserted together.
#[test]
fn a_jim_document_keeps_the_surface_jim_does_have() {
    for name in IN_JIM {
        assert!(
            jim_offers(name),
            "`{name}` is in jimsh and must still resolve"
        );
    }
}

/// The roster narrows **one** family. Tcl's own surface is untouched —
/// the filter only ever applies to an ancestor row reaching a
/// reimplementing descendant, never to a family's own provider.
#[test]
fn the_roster_does_not_narrow_the_ancestor_it_enumerates() {
    for name in NOT_IN_JIM {
        assert!(
            tcl_offers(name),
            "`{name}` is Tcl 8.6's own and must be unaffected by jim's roster"
        );
    }
}

/// §6.3's round-trip direction, for the new word: a pack carrying
/// `include from` rows exports as canonical source that loads back to the
/// same rosters. This is the gate that would catch a ratified word with
/// no export arm — the `object_class` incident's shape.
#[test]
fn a_roster_row_survives_export_and_reload() {
    let source = "speclib probe 2.0 {\n \
                  include from tcl into jim {set proc}\n \
                  include from tcl into jim -available {0.77-} {interp}\n}";
    let pack = tcl_spectcl::loader::evaluate_pack(source);
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert_eq!(pack.surface_rosters.len(), 2);

    let exported = tcl_spectcl::export::export_pack(&pack);
    let reloaded = tcl_spectcl::loader::evaluate_pack(&exported);
    assert_eq!(reloaded.surface_rosters, pack.surface_rosters);
}

#[test]
fn rooted_source_metadata_uses_the_selected_jim_roster_descriptor() {
    use tcl_dialect::{
        DialectProfile,
        model::{DialectPoint, InvocationRealm, Release},
    };
    tcl_spectcl::core_surfaces::ensure();
    let logical = DialectProfile::projected_from_point(
        "jim-rooted-roster-logical079",
        &[],
        "Rooted Logical Jim source metadata",
        DialectPoint::canonical(Release::JIM_0_79),
    )
    .intern();
    let modern = resolve_environment("jim").unit_profile();
    for profile in [logical, modern] {
        let generation = tcl_registry::model::ingress::context_for_profile(profile);
        let registry = generation.commands();
        let context = generation.context();
        for realm in [
            InvocationRealm::RuleLoader,
            InvocationRealm::InterpreterRuntime,
        ] {
            for name in ["if", "proc", "set"] {
                let bare = context
                    .resolve_spec_in_realm(registry, name, realm)
                    .unwrap();
                let rooted = context
                    .resolve_spec_in_realm(registry, &format!("::{name}"), realm)
                    .unwrap();
                assert!(std::ptr::eq(bare, rooted), "{name}");
            }
            for name in ["coroutine", "trace", "yieldto"] {
                assert!(
                    context
                        .resolve_spec_in_realm(registry, name, realm)
                        .is_none()
                );
                assert!(
                    context
                        .resolve_spec_in_realm(registry, &format!("::{name}"), realm)
                        .is_none()
                );
            }
        }
    }
}

#[test]
fn an_exact_rooted_descriptor_keeps_its_own_inherited_roster_key() {
    static ROOTED_IF: tcl_registry::CommandSpec = tcl_registry::CommandSpec {
        name: "::if",
        surface: Some(tcl_dialect::model::SpecSurface::ALL_TCL),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    tcl_spectcl::core_surfaces::ensure();
    let generation = resolve_environment("jim").default_context_registry();
    let context = generation.context();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert_static(&ROOTED_IF);
    assert!(std::ptr::eq(
        registry
            .get_for_surface("::if", Some(context.authoring_query()))
            .unwrap(),
        &ROOTED_IF,
    ));
    assert!(
        context
            .resolve_spec_in_realm(
                &registry,
                "::if",
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .is_none()
    );
}
