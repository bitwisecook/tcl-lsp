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

//! The `package` command — a minimal version registry.
//!
//! Enough to satisfy the `package require Tcl …` / `package provide` /
//! `package vsatisfies` calls at the top of library scripts like `tcltest`.
//! No real on-disk package loading (`ifneeded` scripts) is performed yet.

use std::cmp::Ordering;

use tcl_dialect::{compare_versions as cmp_version, version_satisfies as vsatisfies};
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    provide_core_packages(vm);
    vm.register("package", cmd_package);
}

/// Pre-provide the core's own package entries for the release this VM is
/// pinned to, replacing whatever a previous pin left behind.
///
/// Which names exist and what version each carries is release data
/// ([`tcl_dialect::TclVersion::core_provided_packages`]), not an engine
/// constant: 8.x provides `Tcl` alone while 9.x co-provides the lowercase
/// `tcl` that `tm.tcl`'s version split reads, 8.4 provides the
/// two-component `8.4` rather than a patch level, and `TclOO` arrives at
/// 8.6 one minor version behind 9.x's.
///
/// Called again on every profile pin, so flipping 9.0 → 8.6 withdraws the
/// 9-only names instead of leaving them provided under an 8.x surface.
pub(crate) fn provide_core_packages(vm: &mut Vm) {
    // Withdraw every name *any* release pre-provides before installing this
    // one's. Derived from the same table rather than hand-listed, so a name
    // added to one release's row cannot be left behind by a re-pin.
    for version in tcl_dialect::TclVersion::ALL {
        for core in version.core_provided_packages() {
            vm.forget_package(core.name);
        }
    }
    for core in vm.runtime_version().core_provided_packages() {
        vm.provide_package(core.name, core.version);
    }
}

fn cmd_package(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"package option ?arg ...?\"");
    };
    match &*sub.to_str() {
        "provide" => match rest {
            [name] => ok(vm
                .package_version(&name.to_str())
                .map_or_else(Value::empty, Value::string)),
            [name, version] => {
                vm.provide_package(&name.to_str(), &version.to_str());
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"package provide package ?version?\""),
        },
        "require" | "present" => pkg_require(vm, rest),
        "vsatisfies" => match rest {
            [version, reqs @ ..] if !reqs.is_empty() => {
                let v = version.to_str();
                let satisfied = reqs.iter().any(|r| vsatisfies(&v, &r.to_str()));
                ok(Value::bool(satisfied))
            }
            _ => err(
                "wrong # args: should be \"package vsatisfies version requirement ?requirement ...?\"",
            ),
        },
        "vcompare" => match rest {
            [v1, v2] => {
                let order = cmp_version(&v1.to_str(), &v2.to_str());
                ok(Value::int(match order {
                    Ordering::Less => -1,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                }))
            }
            _ => err("wrong # args: should be \"package vcompare version1 version2\""),
        },
        "names" => {
            let mut names = vm.package_names();
            names.sort();
            ok(Value::list(names.into_iter().map(Value::string).collect()))
        }
        // The registry tracks a single provided version per package, so
        // `versions` reports that one (empty list for an unknown package).
        "versions" => match rest {
            [name] => ok(Value::list(
                vm.package_version(&name.to_str())
                    .map(|v| Value::string(v.to_owned()))
                    .into_iter()
                    .collect(),
            )),
            _ => err("wrong # args: should be \"package versions package\""),
        },
        // Accepted no-ops.
        "ifneeded" | "forget" | "unknown" | "prefer" => ok(Value::empty()),
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
    }
}

fn pkg_require(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    // `-exact NAME VERSION` is the requirement `VERSION-VERSION` — the same
    // rewrite `tclPkg.c`'s `PKG_REQUIRE` arm performs, so exactness needs no
    // second comparison rule (issue #1090).
    let exact = rest.first().is_some_and(|v| &*v.to_str() == "-exact");
    let rest = if exact { &rest[1..] } else { rest };
    let Some((name, reqs)) = rest.split_first() else {
        return err(
            "wrong # args: should be \"package require ?-exact? package ?requirement ...?\"",
        );
    };
    let name = name.to_str();
    let reqs: Vec<String> = if exact {
        reqs.iter()
            .map(|r| tcl_dialect::exact_requirement(&r.to_str()))
            .collect()
    } else {
        reqs.iter().map(|r| r.to_str().to_string()).collect()
    };
    match vm.package_version(&name) {
        Some(v) => {
            let v = v.to_string();
            let ok_version = reqs.is_empty() || reqs.iter().any(|r| vsatisfies(&v, r));
            if ok_version {
                ok(Value::string(v))
            } else {
                err(format!("version conflict for package \"{name}\": have {v}"))
            }
        }
        None => err(format!("can't find package {name}")),
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::TclVersion;

    use super::vsatisfies;
    use crate::interp::Vm;

    /// The pre-provided core packages follow the pinned release, so
    /// `package require Tcl 8.5` fails under a 9.x pin exactly as `tclsh9.0`
    /// fails it (`version conflict for package "Tcl": have 9.0.4, need 8.5`).
    /// Both engines used to hardcode `9.0.4` and provide `Tcl`+`tcl`
    /// regardless of the pin, so that require wrongly *succeeded* (ledger
    /// row B4).
    ///
    /// Measured (`package provide <name>` in a fresh `tclsh`): 8.4.20 →
    /// `Tcl` = `8.4` and no `tcl`; 8.5.19 → `Tcl` = `8.5.19`; 8.6.14 →
    /// `Tcl` = `8.6.14` with `TclOO` = `1.1.0`; 9.0.4 and 9.1b0 → all four
    /// names, at the patch level and `1.3.1`.
    #[test]
    fn core_provides_follow_the_pinned_release() {
        for version in TclVersion::ALL {
            let mut vm = Vm::new();
            vm.set_runtime_version(version);
            for core in version.core_provided_packages() {
                assert_eq!(
                    vm.package_version(core.name),
                    Some(core.version),
                    "{version:?} provides {}",
                    core.name
                );
            }
            // FN guard: a name this release does not pre-provide is absent,
            // not left over from the construction-time default pin (9.0).
            if version < TclVersion::V9_0 {
                assert_eq!(vm.package_version("tcl"), None, "{version:?}");
                assert_eq!(vm.package_version("tcl::oo"), None, "{version:?}");
            }
            if version < TclVersion::V8_6 {
                assert_eq!(vm.package_version("TclOO"), None, "{version:?}");
            }
        }
    }

    /// `package require Tcl 8.5` means `[8.5, 9)`: satisfied on 8.5 and 8.6,
    /// a version conflict on 8.4 and on every 9.x — the tclsh answers
    /// measured on all five reference interpreters.
    #[test]
    fn package_require_tcl_85_matches_tclsh_per_release() {
        for version in TclVersion::ALL {
            let mut vm = Vm::new();
            vm.set_runtime_version(version);
            let provided = vm
                .package_version("Tcl")
                .expect("Tcl is provided")
                .to_owned();
            let satisfied = vsatisfies(&provided, "8.5");
            assert_eq!(
                satisfied,
                matches!(version, TclVersion::V8_5 | TclVersion::V8_6),
                "{version:?}: `package require Tcl 8.5` against provided {provided}"
            );
        }
    }

    #[test]
    fn version_ranges() {
        assert!(vsatisfies("9.0", "8.5-"));
        assert!(vsatisfies("9.0", "9.0-"));
        assert!(!vsatisfies("8.4", "8.5-"));
        assert!(vsatisfies("8.6", "8.5"));
        assert!(!vsatisfies("9.0", "8.5")); // 8.5 → [8.5, 9)
        assert!(vsatisfies("8.5.2", "8.5-9.0"));
        assert!(!vsatisfies("9.0", "8.5-9.0"));
    }
}
