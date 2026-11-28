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

use tcl_cli_support::environment::{context_for_dialect, known_profile_for_dialect};
use tcl_cli_support::resolve_dialect;
use tcl_dialect::model::EnvironmentRegistry;

#[test]
fn explicit_set_only_dialect_resolves_at_cli_ingress() {
    crate::install_surfaces();
    let profile = resolve_dialect(Some("tk"))
        .expect("recognized set-only dialect")
        .expect("an explicit dialect resolves");
    assert_eq!(profile.name, "tk");
    assert!(!profile.is_fallback());
    assert!(profile.hosts_tk());
    // The retired `availability_for_name` validator's answer, taken
    // from the resolved environment's document authoring point.
    assert!(context_for_dialect("tk").authoring_query().carries("Tk"));
}

#[test]
fn unknown_explicit_dialect_is_an_input_error() {
    crate::install_surfaces();
    let error = resolve_dialect(Some("not-a-real-dialect"))
        .expect_err("an unknown spelling must not fall back");
    let message = error.to_string();
    assert!(
        message.starts_with("unknown dialect `not-a-real-dialect`; valid names are "),
        "unexpected message: {message}"
    );
    for environment in EnvironmentRegistry::compiled_selectable() {
        assert!(
            message.contains(environment.id.as_str()),
            "message omits `{}`: {message}",
            environment.id
        );
    }
    assert!(message.contains("jim"), "message omits `jim`: {message}");
    assert!(message.contains("tk"), "message omits `tk`: {message}");
}

/// The language ids an environment lists as selecting it are dialect
/// names too, though no argument parser knows them.
#[test]
fn a_selecting_language_id_resolves() {
    crate::install_surfaces();
    for (name, canonical) in [
        ("tcl-bpf", "bpf"),
        ("tcl-libero", "microchip-libero-eda-tcl"),
        ("tcl-spec", "spectcl"),
        ("tcl-apl", "f5-iapps"),
    ] {
        let profile = resolve_dialect(Some(name))
            .unwrap_or_else(|error| panic!("`{name}` resolves: {error}"))
            .expect("an explicit dialect resolves");
        assert_eq!(
            profile.name,
            known_profile_for_dialect(canonical)
                .expect("the canonical id resolves")
                .name,
            "{name}"
        );
    }
}
