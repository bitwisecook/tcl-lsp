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

//! The editor language ids the ingress accepts.

use super::*;

#[test]
fn dialect_from_language_id_recognises_editor_ids() {
    assert_eq!(
        Backend::dialect_from_language_id("tcl-irule").map(LanguageDialect::name),
        Some("f5-irules"),
    );
    assert_eq!(
        Backend::dialect_from_language_id("tcl-iapp").map(LanguageDialect::name),
        Some("f5-iapps"),
    );
    // APL presentation files are an iApp sublanguage → f5-iapps.
    assert_eq!(
        Backend::dialect_from_language_id("tcl-apl").map(LanguageDialect::name),
        Some("f5-iapps"),
    );
    assert_eq!(
        Backend::dialect_from_language_id("tcl").map(LanguageDialect::name),
        Some("tcl8.6")
    );
}

#[test]
fn dialect_from_language_id_passes_through_canonical_names() {
    assert_eq!(
        Backend::dialect_from_language_id("f5-irules").map(LanguageDialect::name),
        Some("f5-irules"),
    );
    assert_eq!(
        Backend::dialect_from_language_id("tcl9.0").map(LanguageDialect::name),
        Some("tcl9.0"),
    );
    assert_eq!(
        Backend::dialect_from_language_id("synopsys-eda-tcl").map(LanguageDialect::name),
        Some("synopsys-eda-tcl"),
    );
}

/// The `VS Code` extension contributes *undotted* version-pinned language
/// ids (`tcl84` … `tcl91`) because a dotted id cannot carry a
/// `configurationDefaults` override. Every other editor
/// integration still sends the dotted form, so both spellings must resolve
/// to the same dialect.
#[test]
fn dialect_from_language_id_accepts_undotted_and_dotted_version_ids() {
    for (undotted, dotted) in [
        ("tcl84", "tcl8.4"),
        ("tcl85", "tcl8.5"),
        ("tcl86", "tcl8.6"),
        ("tcl90", "tcl9.0"),
        ("tcl91", "tcl9.1"),
    ] {
        assert_eq!(
            Backend::dialect_from_language_id(undotted).map(LanguageDialect::name),
            Some(dotted),
            "undotted VS Code language id `{undotted}` should map to dialect `{dotted}`",
        );
        assert_eq!(
            Backend::dialect_from_language_id(dotted).map(LanguageDialect::name),
            Some(dotted),
            "dotted language id `{dotted}` must keep working for non-VS Code editors",
        );
    }
}

#[test]
fn dialect_from_language_id_returns_none_for_unknown_ids() {
    assert!(Backend::dialect_from_language_id("plaintext").is_none());
    assert!(Backend::dialect_from_language_id("").is_none());
    assert!(Backend::dialect_from_language_id("irules").is_none());
}

/// `f5-bigip` is a catalogue profile like any other, so the catalogue-first
/// lookup answers for its ids. `dialect_for_open_sync` never reaches this
/// function for them — its BIG-IP branch returns first — and resolves them
/// to the same `f5-bigip` either way.
#[test]
fn dialect_from_language_id_resolves_the_bigip_ids_like_dialect_for_open() {
    for id in ["tcl-bigip", "f5-bigip"] {
        assert_eq!(
            Backend::dialect_from_language_id(id).map(LanguageDialect::name),
            Some("f5-bigip"),
            "`{id}` names the BIG-IP profile",
        );
    }
}

/// The full input set the language-id ingress accepts, with the dialect each
/// input resolves to. The ingress is registry-driven (canonical ids,
/// contributed editor identities and the selecting spellings), and every
/// one of these inputs must resolve exactly as listed.
#[test]
fn dialect_from_language_id_accepts_every_legacy_spelling() {
    for (language_id, dialect) in [
        ("tcl", "tcl8.6"),
        ("tcl8.6", "tcl8.6"),
        ("tcl86", "tcl8.6"),
        ("tcl8.4", "tcl8.4"),
        ("tcl84", "tcl8.4"),
        ("tcl8.5", "tcl8.5"),
        ("tcl85", "tcl8.5"),
        ("tcl9.0", "tcl9.0"),
        ("tcl90", "tcl9.0"),
        ("tcl9.1", "tcl9.1"),
        ("tcl91", "tcl9.1"),
        ("tcl-irule", "f5-irules"),
        ("f5-irules", "f5-irules"),
        ("tcl-iapp", "f5-iapps"),
        ("f5-iapps", "f5-iapps"),
        ("tcl-apl", "f5-iapps"),
        ("tcl-tmsh", "f5-tmsh"),
        ("f5-tmsh", "f5-tmsh"),
        ("tcl-bpf", "bpf"),
        ("bpf", "bpf"),
        ("tcl-expect", "expect"),
        ("expect", "expect"),
        ("tcl-synopsys", "synopsys-eda-tcl"),
        ("synopsys-eda-tcl", "synopsys-eda-tcl"),
        ("tcl-cadence", "cadence-eda-tcl"),
        ("cadence-eda-tcl", "cadence-eda-tcl"),
        ("tcl-xilinx", "xilinx-eda-tcl"),
        ("xilinx-eda-tcl", "xilinx-eda-tcl"),
        ("tcl-quartus", "intel-quartus-eda-tcl"),
        ("intel-quartus-eda-tcl", "intel-quartus-eda-tcl"),
        ("tcl-mentor", "mentor-eda-tcl"),
        ("mentor-eda-tcl", "mentor-eda-tcl"),
        ("tcl-libero", "microchip-libero-eda-tcl"),
        ("tcl-microchip", "microchip-libero-eda-tcl"),
        ("microchip-libero-eda-tcl", "microchip-libero-eda-tcl"),
        ("tclspec", "spectcl"),
        ("tcl-spec", "spectcl"),
        ("spectcl", "spectcl"),
        ("sslictcl", "sslictcl"),
        ("tcl-jim", "jim"),
        ("jim", "jim"),
        ("tk", "tk"),
    ] {
        assert_eq!(
            Backend::dialect_from_language_id(language_id).map(LanguageDialect::name),
            Some(dialect),
            "language id `{language_id}` must still resolve to `{dialect}`",
        );
    }
}

/// Every selectable environment answers to its canonical id, its
/// contributed editor identity and the language ids it lists as selecting
/// it, and to nothing that is only an alias — so an environment added
/// later needs no table here.
#[test]
fn dialect_from_language_id_covers_every_selectable_environment() {
    for environment in tcl_registry::model::selectable_environments() {
        let id = environment.id.as_str();
        assert_eq!(
            Backend::dialect_from_language_id(id).map(LanguageDialect::name),
            Some(id),
            "canonical id `{id}` must resolve to itself",
        );
        if let Some(identity) = environment.editor_identity {
            assert_eq!(
                Backend::dialect_from_language_id(identity.as_str()).map(LanguageDialect::name),
                Some(id),
                "editor language id `{}` must resolve to `{id}`",
                identity.as_str(),
            );
        }
        for identity in &environment.selecting_identities {
            assert_eq!(
                Backend::dialect_from_language_id(identity.as_str()).map(LanguageDialect::name),
                Some(id),
                "selecting language id `{}` must resolve to `{id}`",
                identity.as_str(),
            );
        }
        for alias in &environment.aliases {
            let is_identity = environment
                .editor_identity
                .is_some_and(|identity| identity.as_str() == alias.as_ref());
            let is_selecting = environment
                .selecting_identities
                .iter()
                .any(|identity| identity.as_str() == alias.as_ref());
            if !is_identity && !is_selecting {
                assert!(
                    Backend::dialect_from_language_id(alias).is_none(),
                    "alias `{alias}` of `{id}` is not a language id",
                );
            }
        }
    }
}

/// The unknown-dialect rejection names the selectable set, so a caller can
/// fix the spelling from the error alone.
#[test]
fn unknown_dialect_error_lists_every_selectable_name() {
    let message = unknown_dialect_error("tcl8.7");
    assert!(
        message.starts_with("unknown dialect: tcl8.7"),
        "the rejected name comes first: {message}",
    );
    let listed = message
        .split_once("(valid surface: ")
        .and_then(|(_, list)| list.strip_suffix(')'))
        .expect("the message carries the valid surface");
    let expected: Vec<String> = tcl_registry::model::selectable_environments()
        .iter()
        .map(|environment| environment.id.to_string())
        .collect();
    assert_eq!(listed.split(", ").collect::<Vec<_>>(), expected);
}

/// Every list the server builds is the registry's selectable set:
/// `listDialects` and the unknown-dialect rejection name the same canonical
/// ids, in the same order, with nothing added or dropped.
#[test]
fn every_runtime_enumeration_is_the_registry() {
    let expected: Vec<String> = tcl_registry::model::selectable_environments()
        .iter()
        .map(|environment| environment.id.to_string())
        .collect();
    let listed = Backend::list_dialects_command();
    let listed: Vec<&str> = listed
        .as_array()
        .expect("an array of dialects")
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert_eq!(listed, expected);
    let rejection = unknown_dialect_error("klingon");
    let quoted = rejection
        .split_once("(valid surface: ")
        .and_then(|(_, list)| list.strip_suffix(')'))
        .expect("the rejection carries the valid surface");
    assert_eq!(quoted.split(", ").collect::<Vec<_>>(), expected);
}
