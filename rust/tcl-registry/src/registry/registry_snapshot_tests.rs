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

#[test]
fn descriptor_coordinates_reuse_only_their_actual_snapshot_and_keep_identity() {
    let mut registry = CommandRegistry::build_default();
    let snapshot = registry.snapshot();
    let independent = CommandRegistry::build_default().snapshot();
    let hash = |value: &RegistrySnapshot| {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut state);
        state.finish()
    };
    let before_hash = hash(&snapshot);
    let before_key = snapshot.semantic_key();
    assert!(snapshot.0.descriptor_coordinates.get().is_none());
    let coordinates = snapshot.command_descriptor_coordinates();
    let expected = crate::unique_command_descriptor_coordinates(registry.command_descriptor_keys())
        .map(|(coordinate, key)| (coordinate, key.label().to_owned()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(coordinates.as_ref(), &expected);
    assert!(Arc::ptr_eq(
        &coordinates,
        &registry.snapshot().command_descriptor_coordinates(),
    ));
    assert_eq!(snapshot, independent);
    assert_eq!(snapshot.semantic_key(), before_key);
    assert_eq!(hash(&snapshot), before_hash);
    assert!(!Arc::ptr_eq(
        &coordinates,
        &independent.command_descriptor_coordinates(),
    ));

    // Same labels do not erase a changed selected spec or registry owner.
    let original = registry.get("set").unwrap();
    let mut changed = original.clone();
    changed.traits = crate::Traits::CSE_CANDIDATE;
    registry.insert(changed);
    let current = registry.snapshot();
    assert_ne!(snapshot, current);
    assert_eq!(coordinates, current.command_descriptor_coordinates());
    assert!(!Arc::ptr_eq(
        &coordinates,
        &current.command_descriptor_coordinates(),
    ));
    assert!(std::ptr::eq(
        snapshot.registry().get("set").unwrap(),
        original
    ));
    assert!(!std::ptr::eq(
        current.registry().get("set").unwrap(),
        original
    ));
}

#[test]
fn descriptor_coordinate_cache_preserves_bytes_collisions_and_old_generations() {
    let mut registry = CommandRegistry::build_default();
    for name in [
        "snapshot_coordinate_probe",
        "snapshot::",
        "::::snapshot_probe",
    ] {
        registry.insert(crate::CommandSpec {
            name,
            ..crate::CommandSpec::DEFAULT
        });
    }
    let original = registry.snapshot();
    let before = original.command_descriptor_coordinates();
    assert_eq!(
        before["::snapshot_coordinate_probe"],
        "snapshot_coordinate_probe"
    );
    assert_eq!(before["::snapshot::"], "snapshot::");
    assert_eq!(before["::::snapshot_probe"], "::::snapshot_probe");
    registry.insert(crate::CommandSpec {
        name: "::snapshot_coordinate_probe",
        ..crate::CommandSpec::DEFAULT
    });
    let current = registry.snapshot();
    assert_ne!(original, current);
    assert!(
        !current
            .command_descriptor_coordinates()
            .contains_key("::snapshot_coordinate_probe")
    );
    assert_eq!(
        original.command_descriptor_coordinates()["::snapshot_coordinate_probe"],
        "snapshot_coordinate_probe",
    );
    assert!(Arc::ptr_eq(
        &before,
        &original.command_descriptor_coordinates()
    ));
}

#[test]
fn snapshot_retains_pack_reference_overlay_and_special_variable_axes() {
    // Software cache contract: no native body or lookup is asserted.
    let mut registry = CommandRegistry::build_default();
    let spec = registry.get("set").unwrap();
    let original = registry.snapshot();
    let origin = crate::pack_origin::PackOrigin {
        pack: "SnapshotPack".to_owned(),
        content_hash: 123,
        vocabulary_version: "2.0".to_owned(),
        provenance: tcl_dialect::model::Provenance::WorkspaceTrusted,
    };
    registry.insert_pack_origin(spec, origin.clone());
    let with_origin = registry.snapshot();
    assert_ne!(original, with_origin);
    assert_eq!(original.registry().pack_origin(spec), None);
    assert_eq!(with_origin.registry().pack_origin(spec), Some(&origin));
    let address = std::ptr::from_ref(spec).addr();
    registry.insert_reference_text(spec, Arc::from("exact retained definition"));
    let with_reference = registry.snapshot();
    assert_ne!(with_origin, with_reference);
    assert!(
        !with_origin
            .registry()
            .reference_texts
            .contains_key(&address)
    );
    assert_eq!(
        with_reference.registry().reference_texts[&address].as_ref(),
        "exact retained definition",
    );
    registry.insert_special_var(&crate::special_vars::SPECIAL_VARS[0]);
    let with_variable = registry.snapshot();
    assert_ne!(with_reference, with_variable);
    assert_eq!(with_reference.registry().special_vars.len(), 0);
    assert_eq!(with_variable.registry().special_vars.len(), 1);
    registry.set_overlay(123);
    let with_overlay = registry.snapshot();
    assert_ne!(with_variable, with_overlay);
    assert_eq!(with_variable.registry().overlay, None);
    assert_eq!(with_overlay.registry().overlay, Some(123));
    assert_eq!(with_overlay.registry().generation(), registry.generation());
}

#[test]
fn snapshot_retains_original_surface_across_every_mutation_kind() {
    let mut registry = CommandRegistry::build_default();
    let original = registry.snapshot();
    let retained_store = original.shared_registry();
    assert!(std::ptr::eq(retained_store.as_ref(), original.registry()));
    assert!(std::ptr::eq(
        original.registry(),
        registry.snapshot().registry()
    ));
    assert_eq!(original, CommandRegistry::build_default().snapshot());
    registry.insert_ambient_package("SnapshotPackage", "1.0");
    let ambient = registry.snapshot();
    assert_ne!(original, ambient);
    assert!(
        !original
            .registry()
            .ambient_package_rows()
            .contains(&("SnapshotPackage", "1.0"))
    );
    assert!(
        ambient
            .registry()
            .ambient_package_rows()
            .contains(&("SnapshotPackage", "1.0"))
    );
    let mut spec = registry.get("set").unwrap().clone();
    spec.traits = crate::Traits::CSE_CANDIDATE;
    registry.insert(spec);
    let overridden = registry.snapshot();
    assert_ne!(ambient, overridden);
    assert_ne!(
        ambient.registry().get("set").unwrap().traits,
        overridden.registry().get("set").unwrap().traits
    );
    registry.set_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
    let profiled = registry.snapshot();
    assert_ne!(overridden, profiled);
    assert!(overridden.registry().profile().is_none());
    assert_eq!(profiled.registry().profile().unwrap().name, "tcl8.4");
    assert!(retained_store.profile().is_none());
    assert!(
        !retained_store
            .ambient_package_rows()
            .contains(&("SnapshotPackage", "1.0"))
    );
    assert_eq!(
        retained_store.get("set").unwrap().traits,
        original.registry().get("set").unwrap().traits
    );
}

#[test]
fn equal_live_snapshots_share_one_semantic_key() {
    let first = CommandRegistry::build_default().snapshot();
    let second = CommandRegistry::build_default().snapshot();
    assert!(Arc::ptr_eq(
        &first.0.semantic_key.0,
        &second.0.semantic_key.0
    ));
    let mut registry = CommandRegistry::build_default();
    registry.insert_ambient_package("SnapshotShare", "1.0");
    let different = registry.snapshot();
    assert!(!Arc::ptr_eq(
        &first.0.semantic_key.0,
        &different.0.semantic_key.0
    ));
    assert_ne!(first.semantic_key(), different.semantic_key());
}

#[test]
fn equal_hashes_never_make_different_snapshot_contents_equal() {
    let first = CommandRegistry::build_default().snapshot();
    let mut registry = CommandRegistry::build_default();
    registry.insert_ambient_package("SnapshotCollision", "2.0");
    let mut second = registry.snapshot();
    drop(registry);
    let second_data = Arc::get_mut(&mut second.0).expect("unique test snapshot");
    // The interner holds only a weak reference, so the strong owner can
    // still be unwrapped and re-sealed under the colliding fingerprint.
    let shared = std::mem::replace(&mut second_data.semantic_key, first.0.semantic_key.clone());
    let mut data = Arc::try_unwrap(shared.0).expect("unique semantic key");
    data.fingerprint = first.0.semantic_key.0.fingerprint;
    second_data.semantic_key = RegistrySemanticKey(Arc::new(data));
    assert_ne!(first, second);
}

#[test]
fn independently_built_equal_stores_share_one_semantic_key() {
    let first = CommandRegistry::build_default().snapshot();
    let second = CommandRegistry::build_default().snapshot();
    assert!(!Arc::ptr_eq(&first.0, &second.0));
    assert!(Arc::ptr_eq(
        &first.semantic_key().0,
        &second.semantic_key().0
    ));
    assert_eq!(first, second);
}
