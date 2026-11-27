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

/// The indexed registration lookup and compiler selection each equal their
/// full scan for every spec, member, implementation and stock-worker slot,
/// on every dialect.
///
/// Every distinct identity is compared once per registry and dialect. The
/// full scan is taken once per registry and dialect and filtered per
/// identity, and the dialects run on their own threads.
#[test]
fn the_prerequisite_index_selects_what_the_full_scan_selects() {
    // Independent of the environment; built once and shared.
    let default_registry = super::CommandRegistry::build_default();
    let compared: usize = std::thread::scope(|scope| {
        [
            "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim", "irules",
        ]
        .map(|environment| {
            let default_registry = &default_registry;
            scope.spawn(move || compare_index_with_full_scan(environment, default_registry))
        })
        .into_iter()
        .map(|worker| worker.join().expect("comparison thread"))
        .sum()
    });
    assert!(compared > 1000, "{compared} identities compared");
}

fn compare_index_with_full_scan(
    environment: &str,
    default_registry: &super::CommandRegistry,
) -> usize {
    let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
    let dialect = tcl_native_model::InvocationDialect::of_profile(profile);
    let mut compared = 0;
    for registry in [
        crate::model::ingress::static_context_for_profile(profile).commands(),
        default_registry,
    ] {
        let mut identities = std::collections::BTreeSet::<String>::new();
        for specs in registry.by_name.values() {
            for spec in specs {
                identities.insert(spec.name.to_owned());
                identities.insert(tcl_syntax::naming::root_unrooted_key(spec.name));
                let compilations =
                    spec.native_compilation
                        .into_iter()
                        .chain(spec.subcommands.iter().flat_map(|member| {
                            member.native_compilation.into_iter().chain(
                                member
                                    .sub_subcommands
                                    .iter()
                                    .filter_map(|worker| worker.native_compilation),
                            )
                        }));
                for compilation in compilations {
                    for lookup in compilation
                        .implementation_prerequisites(dialect)
                        .unwrap_or_default()
                    {
                        identities.insert(lookup.slot.to_owned());
                    }
                }
                for contract in super::successful_handler_contracts(spec) {
                    for lookup in contract.stock_native_workers(dialect) {
                        identities.insert(lookup.slot.to_owned());
                    }
                }
            }
        }
        let scan = registry.native_registration_full_scan(dialect);
        let lookup_scan = registry.native_registration_lookup_scan(dialect);
        for identity in identities {
            assert_eq!(
                registry.native_registration_lookup_uncached(&identity, dialect),
                super::CommandRegistry::native_registration_lookup_full_scan(
                    &identity,
                    dialect,
                    &lookup_scan
                ),
                "{environment}: registration lookup for {identity}"
            );
            assert_eq!(
                registry.native_compilation_for_registration_uncached(&identity, dialect),
                registry.native_compilation_for_registration_full_scan(&identity, dialect, &scan),
                "{environment}: {identity}"
            );
            compared += 1;
        }
    }
    compared
}
