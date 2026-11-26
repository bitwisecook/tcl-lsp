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

//! The value-transfer contract tests that need the shipped `.tclspec` packs
//! or a pack loaded from `SpecTcl` text (`docs/design/compiler/value-transfers.md`):
//! the set of specs carrying each route, swept over every loadable dialect
//! and the shipped packs, so a route cannot appear, vanish, or move without
//! this file changing beside it, and the implementations a pack declares.
//! The rest of the contract is `tcl-registry`'s own `value_transfers` tests.

use std::collections::BTreeSet;

use tcl_registry::value_transfer::{
    Budget, CommandSemantics, CompletionSupport, DeclaredInput, DeclineReason, EvalAnswer,
    EvalRoute, EvaluatorOwner, Exactness, ExistenceOutcome, FactDomain, HostKind,
    ImplementationBudget, NativeEvalId, NoRouteReason, OperandId, SemanticsDeclaration, TargetId,
    TransferAnswer, resolve_semantics,
};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

mod cell_updates;
mod declared_implementations;
mod derived_transfers;
mod pinned_route_stamps;
mod route_identity;

use pinned_route_stamps::pinned_route_stamps;
use route_identity::route_stamps;

const LOADABLE_DIALECTS: &[&str] = &[
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "f5-irules",
    "f5-iapps",
    "expect",
    "bpf",
];

fn full_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::build_default();
    for name in LOADABLE_DIALECTS {
        let profile = tcl_dialect::DialectProfile::find(name).expect("a compiled-in dialect name");
        for &layer in profile.base_layers {
            reg.load_surface(layer);
        }
    }
    let packs = tcl_spectcl::bundled::load_from(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs"),
    );
    assert!(
        !packs.is_empty(),
        "the shipped EDA loadables must be present"
    );
    for pack in &packs.packs {
        for command in &pack.commands {
            reg.insert(command.spec.clone());
        }
    }
    reg
}

/// Shipped builtins stay on the direct route
/// (`docs/design/compiler/value-transfers-migration.md`): a workspace pack
/// declaring evaluators of its own moves no shipped route. Installing the
/// value-transfer design's executable example over every loadable
/// dialect and the shipped packs adds exactly its three spellings, each on
/// the implementation route; every shipped stamp is still the pinned set's,
/// and every direct route is still the registry's own.
#[test]
fn shipped_builtins_stay_on_the_direct_route() {
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/tenant.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        include_str!("../../../tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec")
            .to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    let mut reg = full_registry();
    for pack in &packs.packs {
        for command in &pack.commands {
            reg.insert(command.spec.clone());
        }
    }
    let actual = route_stamps(&reg);
    let pinned = pinned_route_stamps();
    let moved: Vec<_> = pinned.difference(&actual).collect();
    assert!(moved.is_empty(), "a shipped route moved: {moved:?}");
    let added: Vec<(&str, &str, &str)> = actual
        .difference(&pinned)
        .map(|(name, route, owner)| (name.as_str(), *route, *owner))
        .collect();
    assert_eq!(
        added,
        [
            ("tenant label", "implementation", "-"),
            ("tenant::label", "implementation", "-"),
            ("tenant::tag", "implementation", "-"),
        ]
    );
    assert!(
        actual
            .iter()
            .filter(|(_, route, _)| route.starts_with("direct:"))
            .all(|(_, _, owner)| *owner == "registry"),
        "{actual:#?}"
    );
}
