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

//! Every shipped spec knows which `commands/<pack>/` module declares it.
//!
//! Provenance is what a spec author navigates by, so a command with no pack
//! is a command the studio's pack browser would drop on the floor.

use tcl_dialect::DialectProfile;
use tcl_registry::commands::SPEC_PACKS;
use tcl_registry::registry::{spec_pack_of, spec_packs_of};

/// A pack id is the directory name, so it has to be a legal one — and the
/// table may not list the same directory twice.
#[test]
fn pack_ids_are_unique_directory_names() {
    let mut seen = Vec::new();
    for pack in SPEC_PACKS {
        assert!(
            !seen.contains(&pack.id),
            "{} is listed twice in SPEC_PACKS",
            pack.id
        );
        assert!(
            pack.id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "{} is not a directory name",
            pack.id
        );
        assert!(!pack.label.is_empty(), "{} has no label", pack.id);
        assert!(
            pack.blurb.len() > 20,
            "{}'s blurb is too short to say anything",
            pack.id
        );
        seen.push(pack.id);
    }
}

/// The directory a pack names really exists, so the studio's rendered path
/// points somewhere a contributor can put the file.
#[test]
fn every_pack_names_a_real_module_directory() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands");
    for pack in SPEC_PACKS {
        let dir = std::path::Path::new(root).join(pack.id);
        assert!(
            dir.is_dir(),
            "SPEC_PACKS names {}, but {} is not a directory",
            pack.id,
            dir.display()
        );
    }
}

/// The load-bearing one: in every dialect the studio browses, every command
/// resolves to a pack, and that pack is one of the declared rows.
#[test]
fn every_shipped_command_in_every_dialect_has_a_pack() {
    let ids: Vec<&str> = SPEC_PACKS.iter().map(|pack| pack.id).collect();
    for profile in DialectProfile::all() {
        let registry = tcl_registry::model::static_context_for(profile.name).commands();
        for name in registry.command_names() {
            let Some(spec) = registry.get(name) else {
                continue;
            };
            let pack = spec_pack_of(spec).unwrap_or_else(|| {
                panic!(
                    "{name} has no authoring pack in the {} dialect",
                    profile.name
                )
            });
            assert!(
                ids.contains(&pack),
                "{name} claims pack {pack}, which is not in SPEC_PACKS"
            );
        }
    }
}

/// A dialect that re-declares a core command is filed under the pack whose
/// spec it actually registered, not under a fixed by-name winner.
#[test]
fn a_redeclared_command_is_filed_where_the_dialect_registered_it() {
    let tcl = tcl_registry::model::static_context_for("tcl9.0").commands();
    let irules = tcl_registry::model::static_context_for("f5-irules").commands();

    let core_close = tcl.get("close").expect("Tcl 9.0 has close");
    let irules_close = irules.get("close").expect("iRules has close");
    assert_eq!(spec_pack_of(core_close), Some("tcl"));
    assert_eq!(spec_pack_of(irules_close), Some("irules"));

    // Both declarations are still reported for the name itself.
    let packs = spec_packs_of("close");
    assert!(
        packs.contains(&"tcl"),
        "close is declared in tcl: {packs:?}"
    );
    assert!(
        packs.contains(&"irules"),
        "close is declared in irules: {packs:?}"
    );
}

/// A name no shipped pack declares has no provenance to report.
#[test]
fn a_pack_loaded_name_has_no_authoring_pack() {
    assert!(spec_packs_of("definitely::not::a::command").is_empty());
}
