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

//! **D17 — the EDA environment shells live in their packs.** The six
//! `specs/eda_*.tclspec` packs declare `environment` blocks; the compiled
//! `EDA_SHELLS` table that used to seed the same six definitions is gone.
//!
//! Three things hold that move honest:
//!
//! 1. **Parity with what the compiled shells produced.** The fixture beside
//!    this file is the `Debug` rendering of `eda_environments()` captured
//!    before the table was deleted (provenance rewritten to the bundled
//!    tier, which is what the packs declare at). Every name, alias, editor
//!    identity, extension, display name, help term, base release, ceiling,
//!    and placement the shells carried must load from the packs
//!    field-for-field.
//! 2. **The compiled seed is the packs' projection.** `cargo xtask
//!    gen-bundled-environments` writes the blocks into `tcl-dialect`'s
//!    compiled registry so the environments resolve at generation 0; the
//!    seed and the loaded blocks must agree, and the generator's `--check`
//!    is the drift gate on the committed file.
//! 3. **Trust.** The bundled tier restates its own environments on every
//!    publish; a workspace pack claiming one of their names is refused with
//!    the bundled owner named, and the bundled definition keeps resolving.

use std::path::PathBuf;

use tcl_dialect::model::{EnvironmentDefinition, EnvironmentRegistry, Provenance};
use tcl_registry::model::EnvironmentRegistrationError;
use tcl_spectcl::registration::register_pack_set;
use tcl_spectcl::{PackEnvironmentTier, Tier, bundled};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tcl-spectcl lives at <root>/rust/tcl-spectcl")
        .to_path_buf()
}

/// The environments the shipped packs declare, converted at the bundled
/// tier and sorted by id — the order the compiled table listed them in.
fn pack_declared() -> Vec<EnvironmentDefinition> {
    let set = bundled::load_from(&repo_root().join("specs"));
    assert!(
        set.notices.is_empty(),
        "the shipped packs load without a notice: {:?}",
        set.notices
    );
    let mut definitions: Vec<EnvironmentDefinition> = set
        .packs
        .iter()
        .flat_map(|pack| pack.environments.iter())
        .filter(|environment| !environment.extends)
        .map(|environment| environment.to_definition(PackEnvironmentTier::Bundled))
        .collect();
    definitions.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    definitions
}

#[test]
fn the_packs_declare_exactly_what_the_compiled_shells_declared() {
    let fixture = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eda_environment_shells.txt"),
    )
    .expect("the captured shell snapshot is beside the tests");
    let loaded = format!("{:#?}\n", pack_declared());
    assert!(
        loaded == fixture,
        "the pack-declared EDA environments differ from the compiled shells' snapshot:\n{}",
        unified_diff(&fixture, &loaded)
    );
}

#[test]
fn the_compiled_seed_is_the_packs_projection() {
    let compiled = EnvironmentRegistry::compiled();
    let declared = pack_declared();
    assert_eq!(declared.len(), 6, "the six vendor shells");
    for definition in &declared {
        let id = definition.id.as_str();
        let seeded = compiled
            .resolve(id)
            .unwrap_or_else(|| panic!("{id}: seeded into the compiled registry"));
        assert_eq!(
            *seeded, *definition,
            "{id}: run `cargo xtask gen-bundled-environments`"
        );
        assert_eq!(seeded.provenance, Provenance::BundledPack, "{id}");
    }
    let mut seeded: Vec<&str> = compiled
        .definitions()
        .iter()
        .filter(|definition| definition.provenance == Provenance::BundledPack)
        .map(|definition| definition.id.as_str())
        .collect();
    seeded.sort_unstable();
    let declared_ids: Vec<&str> = declared.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(seeded, declared_ids, "nothing seeded that no pack declares");
}

/// A workspace pack cannot take an EDA name: the set registers with the
/// bundled definition in charge and the intruder named in the rejection.
#[test]
fn a_workspace_pack_cannot_hijack_a_bundled_environment_name() {
    let mut files = tcl_spectcl::discover(&tcl_spectcl::DiscoveryOptions {
        bundled_dir: Some(repo_root().join("specs")),
        skip_user_tier: true,
        ..tcl_spectcl::DiscoveryOptions::default()
    });
    let dir = std::env::temp_dir().join(format!(
        "tcl-lsp-eda-hijack-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let intruder = dir.join("intruder.tclspec");
    std::fs::write(
        &intruder,
        "speclib intruder 2.0 {\n\
         environment xilinx-eda-tcl {\n\
         \x20   display_name {Not Vivado}\n\
         \x20   core tcl 9.0\n\
         \x20   ambient NotVivado keyed ToolVersion\n\
         }\n\
         command not_vivado { arity 0 }\n\
         }\n",
    )
    .expect("write the intruder");
    files.push(tcl_spectcl::PackFile {
        tier: Tier::Workspace,
        path: intruder,
        origin: tcl_spectcl::discovery::Origin::DotDir,
    });
    let set = tcl_spectcl::pack::load(&files);
    assert!(
        set.packs.iter().any(|pack| pack.name == "intruder"),
        "the workspace pack itself loads — the name claim is refused at registration"
    );

    let registration = register_pack_set(&set);
    let refused = registration
        .rejected
        .iter()
        .find(|rejection| rejection.pack == "intruder")
        .expect("the intruder is the rejected pack");
    match &refused.error {
        EnvironmentRegistrationError::Reserved {
            name,
            claimed_by,
            provenance,
        } => {
            assert_eq!(name, "xilinx-eda-tcl");
            assert_eq!(claimed_by, "xilinx-eda-tcl");
            assert_eq!(*provenance, Provenance::WorkspaceTrusted);
        }
        other => panic!("expected a reserved-name refusal, got {other:?}"),
    }
    assert_eq!(
        registration.rejected.len(),
        1,
        "the bundled packs register: {:?}",
        registration.rejected
    );

    // The bundled definition is the one that resolves, from the bundled tier.
    let resolved = tcl_registry::model::resolve_environment("xilinx-eda-tcl");
    assert_eq!(resolved.definition.display_name.as_ref(), "Xilinx EDA Tcl");
    assert_eq!(resolved.definition.provenance, Provenance::BundledPack);
    assert!(
        resolved
            .definition
            .expected_packages
            .iter()
            .any(|placement| placement.package.as_ref() == "vivado")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A minimal line diff for the parity failure message.
fn unified_diff(expected: &str, actual: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let expected: Vec<&str> = expected.lines().collect();
    let actual: Vec<&str> = actual.lines().collect();
    let mut shown = 0_usize;
    for (index, (want, got)) in expected.iter().zip(actual.iter()).enumerate() {
        if want != got {
            let _ = writeln!(out, "line {}:\n- {want}\n+ {got}", index + 1);
            shown += 1;
            if shown >= 20 {
                out.push_str("…\n");
                break;
            }
        }
    }
    if expected.len() != actual.len() {
        let _ = writeln!(
            out,
            "({} fixture lines, {} loaded lines)",
            expected.len(),
            actual.len()
        );
    }
    out
}
