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

//! Contract tests for the versioned-library axis (design doc §7.1, D5):
//! profile library pins × registry spec `min_version` data.

use tcl_dialect::{DialectProfile, LibraryVersionOverrides};
use tcl_registry::{ProfileQueries, registry_for_dialect};

/// The data lock binding hover prose to structured version data: every
/// spec whose hover snippet declares "Introduced in BIG-IP X.Y.Z" must
/// carry exactly that `min_version` and belong to its profile's keyed F5
/// surface package — and vice versa, every F5-package spec with a
/// `min_version` must state its introduction in prose. Prose and data can
/// never drift apart, in either direction.
#[test]
fn introduced_in_big_ip_prose_matches_min_version_data() {
    let marker = "Introduced in BIG-IP ";
    for dialect in ["f5-irules", "f5-iapps"] {
        let reg = registry_for_dialect(dialect);
        for name in reg.command_names() {
            for spec in reg.specs(name) {
                let prose_version = spec.hover.as_ref().and_then(|h| {
                    let idx = h.snippet.find(marker)?;
                    let rest = &h.snippet[idx + marker.len()..];
                    let end = rest
                        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                        .unwrap_or(rest.len());
                    Some(rest[..end].trim_end_matches('.'))
                });
                if let Some(version) = prose_version {
                    assert_eq!(
                        spec.lifecycle.introduced,
                        Some(version),
                        "{}: prose says introduced in {version}, data must agree",
                        spec.name
                    );
                    let pkg = spec.required_package.unwrap_or_else(|| {
                        panic!("{}: versioned F5 spec needs its package", spec.name)
                    });
                    assert!(
                        DialectProfile::by_name(dialect).is_ambient_package(pkg),
                        "{}: {pkg} must be an ambient pin of {dialect}",
                        spec.name
                    );
                } else if let Some(pkg) = spec.required_package
                    && DialectProfile::by_name(dialect).is_ambient_package(pkg)
                {
                    assert_eq!(
                        spec.lifecycle.introduced, None,
                        "{}: an F5-surface min_version needs its introduction stated \
                         in the hover prose (the data lock is bidirectional)",
                        spec.name
                    );
                }
            }
        }
    }
}

/// Subcommand introduction prose is locked to the structured package-version
/// floor just as command-level prose is.
#[test]
fn introduced_in_big_ip_subcommand_prose_matches_min_version_data() {
    let marker = "Introduced in BIG-IP ";
    let profile = DialectProfile::irules();
    let reg = registry_for_dialect("f5-irules");
    for name in reg.command_names() {
        let spec = reg.get(name).expect("registry command");
        for sub in spec.subcommands {
            let prose_version = sub.hover.as_ref().and_then(|hover| {
                let idx = hover.snippet.find(marker)?;
                let rest = &hover.snippet[idx + marker.len()..];
                let end = rest
                    .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                    .unwrap_or(rest.len());
                Some(rest[..end].trim_end_matches('.'))
            });
            assert_eq!(
                prose_version, sub.lifecycle.introduced,
                "{} {}: introduction prose and min_version must agree",
                spec.name, sub.name
            );
            if sub.lifecycle.introduced.is_some() {
                assert!(
                    profile.keyed_pin_for(spec).is_some(),
                    "{} {}: a versioned iRules subcommand needs the keyed BIG-IP axis",
                    spec.name,
                    sub.name
                );
            }
        }
    }
}

#[test]
fn bigip_21_1_irules_subcommands_and_values_are_versioned() {
    let reg = registry_for_dialect("f5-irules");
    let c3d = reg.get("SSL::c3d").expect("SSL::c3d spec");
    for name in ["cert_lifespan", "cert_start_date"] {
        let sub = c3d.resolve_subcommand(name).expect("21.1 C3D subcommand");
        assert_eq!(sub.lifecycle.introduced, Some("21.1.0"));
        assert!(!sub.available_for_version(Some("21.0.0")));
        assert!(sub.available_for_version(Some("21.1.0")));
    }

    let persist = reg.get("persist").expect("persist spec");
    let mcp = persist.resolve_subcommand("mcp").expect("persist mcp");
    assert_eq!(mcp.lifecycle.introduced, Some("21.1.0"));
    assert!(!mcp.available_for_version(Some("21.0.0")));
    assert!(mcp.available_for_version(Some("21.1.0")));
    for operation in ["add", "lookup", "delete"] {
        let sub = persist
            .resolve_subcommand(operation)
            .expect("table operation");
        assert!(!sub.arg_value_available_for_version(0, "mcp", Some("21.0.0")));
        assert!(sub.arg_value_available_for_version(0, "mcp", Some("21.1.0")));
        assert!(sub.arg_value_available_for_version(0, "ssl", Some("21.0.0")));
    }
}

/// D5 floors over the backfilled datum: at the oldest-supported default
/// the 16.1.0-introduced command is available; pinned below it is not;
/// pinned above it is again.
#[test]
fn keyed_default_floor_gates_the_f5_surface() {
    let reg = registry_for_dialect("f5-irules");
    let irules = DialectProfile::irules();
    let spec = reg.get("HTTP2::header").expect("HTTP2::header spec");
    assert_eq!(spec.lifecycle.introduced, Some("16.1.0"));

    let default_floor = irules.library_floor_default("f5-irules-cmds");
    assert_eq!(default_floor, Some("16.1.0"), "D5 oldest-supported default");
    let overridden = LibraryVersionOverrides {
        bigip_version: Some("15.1.0".to_owned()),
        ..LibraryVersionOverrides::default()
    };
    assert_eq!(
        irules.library_floor("f5-irules-cmds", &overridden),
        Some("15.1.0"),
        "a session pin overrides the default"
    );
    // TP: available at the default and above…
    assert!(spec.available_for_version(default_floor));
    assert!(spec.available_for_version(Some("17.1.0")));
    // TN: …not below.
    assert!(!spec.available_for_version(Some("15.1.0")));

    // The availability axis (mask + §9) is untouched by the version axis:
    // the command still resolves under the profile — "resolves but needs a
    // newer BIG-IP" is W135's domain, not unknown-command's.
    assert!(irules.resolve_command(reg, "HTTP2::header").is_some());
    // And ambience: the F5 surface never needs `package require`.
    assert!(irules.is_ambient_package("f5-irules-cmds"));
}

/// Hosted pins refine floors without making the package ambient: Tk on a
/// plain Tcl base tracks the runtime version, and Itcl is pinned.
#[test]
fn hosted_pins_supply_tracking_floors() {
    let none = LibraryVersionOverrides::default();
    for (dialect, tk_floor) in [("tcl8.6", "8.6"), ("tcl9.0", "9.0"), ("tcl8.4", "8.4")] {
        let p = DialectProfile::by_name(dialect);
        assert_eq!(p.library_floor("Tk", &none), Some(tk_floor), "{dialect}");
        assert!(
            !p.is_ambient_package("Tk"),
            "{dialect}: Tk still needs its `package require`"
        );
    }
    // An 8.7-introduced Tk option is below no plain base today, so the
    // tracking floor correctly hides it everywhere ≤ 8.6 and admits it on
    // 9.x (Tk 9.0 carries the 8.7 additions).
    let reg = registry_for_dialect("tcl8.6");
    let entry = reg.get("entry").expect("entry spec");
    let placeholder = entry
        .options
        .iter()
        .find(|o| o.name == "-placeholder")
        .expect("entry -placeholder is declared");
    assert_eq!(placeholder.lifecycle.introduced, Some("8.7"));
    assert!(!placeholder.available_for_version(Some("8.6")));
    assert!(placeholder.available_for_version(Some("9.0")));
}
