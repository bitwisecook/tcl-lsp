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

//! A [`Placement::Family`] surface: its specs reach the stores of the family
//! they have a core row for and no other, and count in the any-dialect
//! universe. The harness installs the shipped list plus a probe surface, so
//! each test installs before its first read.

use tcl_dialect::model::{Family, SpecProvider, SpecSurface};
use tcl_registry::model::resolve_environment;
use tcl_registry::{
    CommandRegistry, CommandSpec, HookTable, Placement, SpecCell, SpecSource, Surface,
    commands::SHIPPED,
};

const PROBE_NAME: &str = "core-surface-probe";

fn probe_specs() -> Vec<&'static CommandSpec> {
    static PROBE: CommandSpec = CommandSpec {
        name: PROBE_NAME,
        surface: Some(&[SpecSurface::core(Family::Jim)]),
        ..CommandSpec::DEFAULT
    };
    vec![&PROBE]
}

static PROBE: Surface = Surface {
    id: "probe",
    label: "probe",
    blurb: "A family surface for the tests.",
    source_dir: "rust/tcl-registry/tests",
    specs: SpecSource::Registered(probe_specs),
    placement: Placement::Family,
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

const fn with_probe<const N: usize>(shipped: &'static [&'static Surface]) -> [&'static Surface; N] {
    let mut surfaces = [&PROBE; N];
    let mut index = 0;
    while index < shipped.len() {
        surfaces[index] = shipped[index];
        index += 1;
    }
    surfaces
}

static SURFACES: [&Surface; SHIPPED.len() + 1] = with_probe(SHIPPED);

fn install() {
    tcl_registry::install(&SURFACES).expect("the probe list installs");
}

#[test]
fn a_family_carries_its_own_surface_and_no_other_family_does() {
    install();
    let jim = resolve_environment("jim").default_context_registry();
    assert!(jim.commands().get(PROBE_NAME).is_some());
    assert!(jim.resolve_command(PROBE_NAME).is_some());
    for environment in ["tcl8.6", "tcl9.0", "tcl", "tk", "f5-irules", "expect"] {
        let generation = resolve_environment(environment).default_context_registry();
        assert!(
            generation.commands().get(PROBE_NAME).is_none(),
            "{environment}: another family's own spec is not in its store"
        );
        assert!(generation.resolve_command(PROBE_NAME).is_none());
    }
    assert!(CommandRegistry::build_default().get(PROBE_NAME).is_none());
}

#[test]
fn the_any_dialect_universe_includes_the_family_surface() {
    install();
    let registry = CommandRegistry::build_default();
    assert!(registry.known_in_any_dialect(PROBE_NAME));
    assert!(registry.known_in_any_dialect("set"), "shipped names stay");
    let offered = registry
        .providers_in_any_dialect(PROBE_NAME)
        .expect("the family name has providers");
    assert_eq!(offered.providers, [SpecProvider::Core(Family::Jim)]);
    assert!(!offered.unrestricted);
    assert!(!registry.known_in_any_dialect("core-surface-unregistered"));
}

#[test]
fn the_family_store_extends_the_plain_store_without_replacing_it() {
    install();
    let plain = resolve_environment("tcl").default_context_registry();
    let jim = resolve_environment("jim").default_context_registry();
    assert!(plain.commands().get(PROBE_NAME).is_none());
    assert!(jim.commands().get(PROBE_NAME).is_some());
    assert!(jim.commands().get("set").is_some());
    assert!(
        jim.commands().get("readFile").is_some(),
        "shared catalogue metadata remains available for assistance"
    );
    assert!(
        jim.resolve_command("readFile").is_none(),
        "catalogue presence does not install an unavailable native/provider command"
    );
    assert!(plain.commands().get("readFile").is_some());
}

#[test]
fn the_installed_family_specs_are_the_registered_rows() {
    let set = tcl_registry::install(&SURFACES).expect("the probe list installs");
    let specs = set.family_specs(Family::Jim);
    assert_eq!(specs.len(), 1);
    assert!(std::ptr::eq(specs[0], probe_specs()[0]));
    assert!(set.family_specs(Family::Tcl).is_empty());
}
