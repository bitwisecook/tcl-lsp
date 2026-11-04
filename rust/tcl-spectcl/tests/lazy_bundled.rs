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

//! The bundled vendor packs load manifest-only until an environment needs
//! them (`tcl_spectcl::demand`), and a manifest is the pack less its commands.
//!
//! Its own test binary: deferral and demand are process-wide.

use std::path::PathBuf;

use tcl_spectcl::{EvalOptions, Pack, evaluate_pack_with};

fn specs() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs")
}

fn pack<'a>(set: &'a tcl_spectcl::PackSet, name: &str) -> &'a tcl_spectcl::MergedPack {
    set.packs
        .iter()
        .find(|pack| pack.name == name)
        .unwrap_or_else(|| panic!("bundled pack {name}"))
}

#[test]
fn vendor_packs_load_manifest_only_until_an_environment_needs_them() {
    tcl_spectcl::demand::enable();

    let cold = tcl_spectcl::bundled::load_from(&specs());
    for merged in &cold.packs {
        assert!(
            merged.commands.is_empty(),
            "{} evaluated commands no environment asked for",
            merged.name
        );
    }
    assert!(
        !pack(&cold, "eda_xilinx").environments.is_empty(),
        "a manifest still declares its environment"
    );
    // A set that installs no command still has a key, and the key is what the
    // analyser asks for: its overlay must be there, or every document reads
    // "analysis context unavailable".
    assert_ne!(cold.key, 0);
    for dialect in ["tcl8.6", "jim", "f5-irules"] {
        let _registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &cold);
        assert!(
            tcl_registry::model::resolve_environment(dialect)
                .context_registry(&tcl_registry::model::KeyedVersions::default(), cold.key)
                .is_ok(),
            "{dialect}: no analysis context under the manifest set's key"
        );
    }

    assert!(!tcl_spectcl::demand::note_dialect("tcl8.6"));
    assert!(tcl_spectcl::demand::note_dialect("xilinx-eda-tcl"));
    assert!(
        !tcl_spectcl::demand::note_dialect("xilinx-eda-tcl"),
        "demand only reports growth"
    );

    let warm = tcl_spectcl::bundled::load_from(&specs());
    assert_ne!(cold.key, warm.key, "what is deferred is part of the key");
    for name in ["eda_xilinx", "sdc_base", "upf"] {
        assert!(
            !pack(&warm, name).commands.is_empty(),
            "{name} now loads in full"
        );
    }
    for name in [
        "eda_cadence",
        "eda_mentor",
        "eda_microchip",
        "eda_quartus",
        "eda_synopsys",
    ] {
        assert!(
            pack(&warm, name).commands.is_empty(),
            "{name} is still not needed"
        );
    }
}

/// Everything but the commands — environments, extension routes, ambient
/// rows, rosters, special variables — reads the same from a manifest as from
/// the full pack, so registering a deferred pack's environments and routes
/// is unchanged by deferral.
#[test]
fn a_manifest_is_the_pack_less_its_commands() {
    let facts = |pack: &Pack| {
        format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            pack.name,
            pack.dsl_version,
            pack.display_name,
            pack.file_extensions,
            pack.provides,
            pack.ambient_packages,
            pack.special_vars,
            pack.environments,
            pack.dialects,
            pack.surface_rosters,
        )
    };
    for row in tcl_spectcl::bundled_manifest::BUNDLED_MANIFESTS {
        let source = std::fs::read_to_string(specs().join(row.file)).expect("bundled pack source");
        let full = evaluate_pack_with(&source, &EvalOptions::default());
        let manifest = evaluate_pack_with(
            &source,
            &EvalOptions {
                commands: false,
                ..EvalOptions::default()
            },
        );
        assert!(manifest.commands.is_empty(), "{}", row.file);
        assert!(manifest.load_error.is_none(), "{}", row.file);
        assert_eq!(facts(&manifest), facts(&full), "{}", row.file);
    }
}

#[test]
fn the_precompiled_manifests_describe_the_shipped_bytes() {
    for row in tcl_spectcl::bundled_manifest::BUNDLED_MANIFESTS {
        let source = std::fs::read_to_string(specs().join(row.file)).expect("bundled pack source");
        assert_eq!(
            row.source_hash,
            tcl_spectcl::demand::source_hash(&source),
            "{} changed — run `cargo xtask gen-bundled-manifests`",
            row.file
        );
        assert_eq!(
            row.source_hash,
            tcl_spectcl::demand::source_hash(&source.replace('\n', "\r\n")),
            "line endings do not change the hash"
        );
    }
}
