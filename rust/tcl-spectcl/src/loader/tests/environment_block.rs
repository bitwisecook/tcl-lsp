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

//! `environment` blocks: conversion to a definition, help terms, ceilings, kinds, reserved names, `-extend` and editor identities.

use super::*;

/// An `environment` block parses, validates, and converts.
#[test]
fn an_environment_block_loads_and_converts_to_a_definition() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment vivado-tcl {\n \
             display_name {Xilinx Vivado}\n core tcl 8.6\n \
             ambient Vivado keyed ToolVersion\n hosted Tk 8.5-\n \
             alias vivado\n editor_identity tcl\n \
             file_extension .XDC -name {Xilinx Design Constraints}\n \
             filename vivado.jou\n signature {create_project}\n \
             policy ambient-plus-require\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert!(pack.command("demo").is_some(), "the pack's commands load");
    let environment = &pack.environments[0];
    assert_eq!(environment.id, "vivado-tcl");
    assert_eq!(environment.aliases, vec!["vivado".to_owned()]);
    assert_eq!(environment.world_policy, WorldPolicy::AmbientPlusRequire);
    assert_eq!(environment.file_extensions[0].extension.as_ref(), "xdc");

    let definition = environment.to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(definition.id.as_str(), "vivado-tcl");
    assert_eq!(definition.display_name.as_ref(), "Xilinx Vivado");
    assert_eq!(definition.provenance, Provenance::WorkspaceTrusted);
    // The editor's trust state is what decides the workspace's class.
    assert_eq!(
        environment
            .to_definition(PackEnvironmentTier::Workspace(
                tcl_dialect::model::WorkspaceTrust::Untrusted,
            ))
            .provenance,
        Provenance::WorkspaceUntrusted
    );
    assert_eq!(
        definition.core.expect("a core selector").default_release,
        Release::TCL_8_6
    );
    assert_eq!(definition.expected_packages.len(), 2);
    assert!(definition.expected_packages[0].ambient);
    assert!(!definition.expected_packages[1].ambient);
    assert_eq!(definition.server_detection.filenames.len(), 1);
    assert_eq!(definition.server_detection.content_signatures.len(), 1);
    assert_eq!(
        definition
            .editor_identity
            .expect("a contributed identity")
            .as_str(),
        "tcl"
    );
    assert_eq!(
        environment
            .to_definition(PackEnvironmentTier::Bundled)
            .provenance,
        Provenance::BundledPack
    );
}

/// `help_terms` and `version_ceiling` (D17: the two facts the compiled
/// EDA shells carried that the block could not yet say) load and
/// convert; the ceiling resolves against the `core` family whichever
/// order the two rows come in.
#[test]
fn help_terms_and_version_ceiling_rows_load_and_convert() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment vivado-tcl {\n \
             version_ceiling 8.5\n core tcl 8.5\n \
             help_terms {Vivado xilinx}\n help_terms {fpga}\n \
             }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let environment = &pack.environments[0];
    assert_eq!(environment.version_ceiling, Some(Release::TCL_8_5));
    assert_eq!(environment.help_terms, ["vivado", "xilinx", "fpga"]);
    let definition = environment.to_definition(PackEnvironmentTier::Bundled);
    assert_eq!(
        definition.policy_defaults.version_ceiling,
        Some(Release::TCL_8_5)
    );
    let terms: Vec<&str> = definition.help_terms.iter().map(AsRef::as_ref).collect();
    assert_eq!(terms, ["vivado", "xilinx", "fpga"]);
}

/// `kind` and `short_name` load and convert; a block that states
/// neither is a base release plus packages, named by its display name.
#[test]
fn kind_and_short_name_rows_load_and_convert() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n \
             environment probe-language {\n display_name {Probe Language}\n \
             short_name {Probe}\n kind language\n core tcl 8.6\n }\n \
             environment probe-shell {\n display_name {Probe Shell}\n \
             core tcl 8.6\n }\n \
             environment probe-bare {\n core tcl 8.6\n kind packages\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert_eq!(pack.environments.len(), 3);

    let language = pack.environments[0].to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(language.kind, EnvironmentKind::Language);
    assert_eq!(language.short_name.as_ref(), "Probe");
    assert_eq!(language.description(), "Probe Language");

    let shell = pack.environments[1].to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(shell.kind, EnvironmentKind::Packages, "packages by default");
    assert_eq!(shell.short_name.as_ref(), "Probe Shell", "the display name");

    let bare = pack.environments[2].to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(bare.kind, EnvironmentKind::Packages);
    assert_eq!(bare.short_name.as_ref(), "probe-bare", "the id");
}

/// An unknown kind word is ignored with a notice: kind is presentation,
/// so the environment still loads and resolves.
#[test]
fn an_unknown_kind_word_is_ignored_with_a_notice() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment probe-shell {\n core tcl 8.6\n \
             kind dialect\n }\n}",
    );
    assert_eq!(pack.environments.len(), 1, "the block still loads");
    assert_eq!(pack.environments[0].kind, EnvironmentKind::Packages);
    assert!(
        pack.notices.iter().any(|n| n
            .message
            .contains("`kind dialect` is not an environment kind")),
        "{:?}",
        pack.notices
    );
}

/// An `-extend` block is additive and may not restate the owner's
/// identity rows, `kind` and `short_name` among them.
#[test]
fn an_extend_block_cannot_restate_kind_or_short_name() {
    for row in ["kind language", "short_name {Probe}"] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{\n environment tcl8.6 -extend {{\n {row}\n }}\n}}"
        ));
        assert!(
            pack.environments.is_empty(),
            "`{row}`: the block is rejected"
        );
    }
}

/// A ceiling off the core's ladder, or with no compiled core to sit on,
/// rejects the block: it is a point on a ladder or it is nothing.
#[test]
fn a_version_ceiling_off_the_ladder_rejects_the_block() {
    for body in ["core tcl 8.5\n version_ceiling 8.7", "version_ceiling 8.5"] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{\n environment vivado-tcl {{\n {body}\n }}\n \
                 command demo {{ arity 1 }}\n}}"
        ));
        assert!(
            pack.environments.is_empty(),
            "{body}: {:?}",
            pack.environments
        );
        assert!(
            pack.command("demo").is_some(),
            "{body}: the rest still loads"
        );
        assert!(
            pack.notices
                .iter()
                .any(|n| n.message.contains("version_ceiling")),
            "{body}: {:?}",
            pack.notices
        );
    }
}

/// §3.3: a compiled canonical name (or alias) is reserved, and a block
/// claiming one is rejected rather than merged.
#[test]
fn an_environment_block_claiming_a_compiled_name_is_rejected() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment tcl8.6 {\n core tcl 8.6\n }\n \
             command demo { arity 1 }\n}",
    );
    assert!(pack.environments.is_empty(), "{:?}", pack.environments);
    assert!(pack.command("demo").is_some(), "the rest still loads");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("a compiled environment name")),
        "{:?}",
        pack.notices
    );
}

/// An unknown row in an `environment` block is semantic-class: the
/// block is rejected, and the pack's other content still loads.
#[test]
fn an_unknown_environment_row_rejects_the_block() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment probe-env {\n core tcl 8.6\n \
             no_such_row yes\n }\n command demo { arity 1 }\n}",
    );
    assert_eq!(
        pack.environments,
        [] as [crate::loader::environment_block::PackEnvironment; 0]
    );
    assert!(pack.command("demo").is_some(), "the rest still loads");
    assert!(
        pack.notices.iter().any(|n| {
            n.class == VocabularyClass::Presentation
                && n.message.contains("semantic-class vocabulary")
        }),
        "{:?}",
        pack.notices
    );
}

/// An `environment NAME -extend { … }` block parses additively: it may
/// name a compiled environment, carries detection rows and placements,
/// and refuses identity rows.
#[test]
fn an_environment_extend_block_is_additive() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment synopsys-eda-tcl -extend {\n \
             file_extension upf -name {Unified Power Format}\n \
             ambient upf_extras 1.0\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let environment = &pack.environments[0];
    assert!(environment.extends);
    assert_eq!(environment.id, "synopsys-eda-tcl");
    assert_eq!(environment.file_extensions[0].extension.as_ref(), "upf");
    assert_eq!(environment.placements.len(), 1);
    let extension = environment.to_extension(PackEnvironmentTier::Bundled);
    assert_eq!(extension.base, "synopsys-eda-tcl");
    assert_eq!(extension.provenance, Provenance::BundledPack);
    assert_eq!(extension.file_extensions.len(), 1);
    assert_eq!(extension.placements.len(), 1);

    // An identity row inside an extend block rejects the block.
    let rejected = evaluate_pack(
        "speclib probe 2.0 {\n environment synopsys-eda-tcl -extend {\n \
             core tcl 8.6\n }\n command demo { arity 1 }\n}",
    );
    assert_eq!(
        rejected.environments,
        [] as [crate::loader::environment_block::PackEnvironment; 0]
    );
    assert!(
        rejected
            .notices
            .iter()
            .any(|n| n.message.contains("identity row")),
        "{:?}",
        rejected.notices
    );
}

/// An unknown editor identity keeps the row and drops only the routing
/// An environment selects from the contributed set.
#[test]
fn an_unknown_editor_identity_drops_only_the_routing() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment probe-env {\n core tcl 8.6\n \
             editor_identity klingon\n }\n}",
    );
    let environment = &pack.environments[0];
    assert_eq!(environment.editor_identity, None);
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("not a contributed editor language id")),
        "{:?}",
        pack.notices
    );
}

/// `selecting_identity` rows carry the contributed language ids that
/// select the environment, and an unknown id is dropped with a notice.
#[test]
fn selecting_identity_rows_carry_contributed_language_ids() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n environment probe-env {\n core tcl 8.6\n \
             editor_identity tcl-microchip\n selecting_identity tcl-libero\n \
             selecting_identity klingon\n }\n}",
    );
    let environment = &pack.environments[0];
    let carried: Vec<&str> = environment
        .selecting_identities
        .iter()
        .map(|identity| identity.as_str())
        .collect();
    assert_eq!(carried, ["tcl-libero"]);
    let definition = environment.to_definition(PackEnvironmentTier::Workspace(
        tcl_dialect::model::WorkspaceTrust::Trusted,
    ));
    assert_eq!(
        definition.selecting_identities,
        environment.selecting_identities
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("`selecting_identity klingon`")),
        "{:?}",
        pack.notices
    );
}
