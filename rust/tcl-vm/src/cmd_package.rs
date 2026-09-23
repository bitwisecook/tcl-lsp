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

//! The `package` command — provided versions plus the standard-library
//! discovery protocol (`unknown` and `ifneeded`).

use std::cmp::Ordering;

use tcl_dialect::{
    PackagePrefer, compare_versions_for as cmp_version, select_package_version_exact_for,
    select_package_version_for, validate_requirement_for, validate_version_for,
    version_matches_exact_for, version_satisfies_for as vsatisfies,
};
use tcl_runtime_api::Completion;

use crate::command::err_with_code;
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

/// `package`'s subcommand words, in C table order (`pkgOptions[]`,
/// `tclPkg.c`). C resolves them with `Tcl_GetIndexFromObj(…, "option", 0)`,
/// so `n` resolves to `names` while `v` and `pr` are ambiguous, and the empty
/// word — a prefix of every entry — is `ambiguous option ""`. 9.0 added
/// `files` at the head of the table.
const PACKAGE_OPTIONS_8X: &[&str] = &[
    "forget",
    "ifneeded",
    "names",
    "prefer",
    "present",
    "provide",
    "require",
    "unknown",
    "vcompare",
    "versions",
    "vsatisfies",
];

/// [`PACKAGE_OPTIONS_8X`] as 8.4 spells it: `prefer` is TIP 268, which 8.5
/// brought in (tclsh8.4.20: `bad option "prefer": must be forget, ifneeded,
/// names, present, provide, …`).
const PACKAGE_OPTIONS_8_4: &[&str] = &[
    "forget",
    "ifneeded",
    "names",
    "present",
    "provide",
    "require",
    "unknown",
    "vcompare",
    "versions",
    "vsatisfies",
];

/// [`PACKAGE_OPTIONS_8X`] as 9.0 spells it.
const PACKAGE_OPTIONS_9X: &[&str] = &[
    "files",
    "forget",
    "ifneeded",
    "names",
    "prefer",
    "present",
    "provide",
    "require",
    "unknown",
    "vcompare",
    "versions",
    "vsatisfies",
];

fn cmd_package(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"package option ?arg ...?\"");
    };
    let options = match vm.runtime_version() {
        tcl_dialect::TclVersion::V8_4 => PACKAGE_OPTIONS_8_4,
        tcl_dialect::TclVersion::V8_5 | tcl_dialect::TclVersion::V8_6 => PACKAGE_OPTIONS_8X,
        _ => PACKAGE_OPTIONS_9X,
    };
    let word = sub.to_str();
    let sub = match tcl_cmd_core::prefix::OptionTable::abbreviating("option", options)
        .index_of(word.as_bytes())
    {
        Ok(i) => options[i],
        Err(message) => {
            return err_with_code(
                String::from_utf8_lossy(&message).into_owned(),
                &tcl_syntax::list::join_list(["TCL", "LOOKUP", "INDEX", "option", &word]),
            );
        }
    };
    match sub {
        "provide" => pkg_provide(vm, rest),
        "require" => pkg_require(vm, rest, true),
        "present" => pkg_require(vm, rest, false),
        "vsatisfies" => pkg_vsatisfies(vm, rest),
        "vcompare" => pkg_vcompare(vm, rest),
        "names" => {
            let mut names = vm.package_names();
            names.sort();
            ok(Value::list(names.into_iter().map(Value::string).collect()))
        }
        "versions" => match rest {
            [name] => {
                let name = name.to_str();
                let mut versions = vm.package_ifneeded_versions(&name);
                if let Some(provided) = vm.package_version(&name)
                    && !versions.iter().any(|version| version == provided)
                {
                    versions.push(provided.to_owned());
                }
                let release = vm.runtime_version();
                versions.sort_by(|left, right| cmp_version(right, left, release));
                ok(Value::list(
                    versions.into_iter().map(Value::string).collect(),
                ))
            }
            _ => err("wrong # args: should be \"package versions package\""),
        },
        "ifneeded" => pkg_ifneeded(vm, rest),
        "unknown" => pkg_unknown(vm, rest),
        "forget" => {
            for name in rest {
                vm.forget_package_completely(&name.to_str());
            }
            ok(Value::empty())
        }
        "prefer" => pkg_prefer(vm, rest),
        // `package files package` (9.0). C reports the files a *loader*
        // recorded for the package; nothing here loads through one, so the
        // answer is the empty list — as it is in tclsh for a package provided
        // by script (`package provide foo 1.0; package files foo` → {}).
        "files" => match rest {
            [_name] => ok(Value::empty()),
            _ => err("wrong # args: should be \"package files package\""),
        },
        // Unreachable: every name in the release's table has an arm above.
        other => err(format!(
            "bad option \"{other}\": must be {}",
            tcl_cmd_core::prefix::choice_list(options)
        )),
    }
}

fn pkg_provide(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [name] => ok(vm
            .package_version(&name.to_str())
            .map_or_else(Value::empty, Value::string)),
        [name, version] => {
            let name = name.to_str().to_string();
            let version = version.to_str().to_string();
            if !validate_version_for(&version, vm.runtime_version()) {
                return invalid_version(&version);
            }
            if let Some(provided) = vm.package_version(&name).map(str::to_owned) {
                if cmp_version(&provided, &version, vm.runtime_version()) != Ordering::Equal {
                    return err_with_code(
                        format!(
                            "conflicting versions provided for package \"{name}\": {provided}, then {version}"
                        ),
                        "TCL PACKAGE VERSIONCONFLICT",
                    );
                }
                return ok(Value::empty());
            }
            vm.provide_package(&name, &version);
            ok(Value::empty())
        }
        _ => err("wrong # args: should be \"package provide package ?version?\""),
    }
}

fn pkg_vsatisfies(vm: &Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [version, reqs @ ..] if !reqs.is_empty() => {
            let v = version.to_str();
            let release = vm.runtime_version();
            if !validate_version_for(&v, release) {
                return invalid_version(&v);
            }
            for requirement in reqs {
                if let Err(error) = validate_requirement_for(&requirement.to_str(), release) {
                    return invalid_requirement(error);
                }
            }
            let satisfied = reqs.iter().any(|r| vsatisfies(&v, &r.to_str(), release));
            ok(Value::bool(satisfied))
        }
        _ => err(
            "wrong # args: should be \"package vsatisfies version requirement ?requirement ...?\"",
        ),
    }
}

fn pkg_vcompare(vm: &Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [v1, v2] => {
            let v1 = v1.to_str();
            let release = vm.runtime_version();
            if !validate_version_for(&v1, release) {
                return invalid_version(&v1);
            }
            let v2 = v2.to_str();
            if !tcl_dialect::validate_version_for(&v2, release) {
                return invalid_version(&v2);
            }
            let order = cmp_version(&v1, &v2, release);
            ok(Value::int(match order {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }))
        }
        _ => err("wrong # args: should be \"package vcompare version1 version2\""),
    }
}

fn pkg_ifneeded(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [name, version] => {
            let version = version.to_str();
            if !validate_version_for(&version, vm.runtime_version()) {
                return invalid_version(&version);
            }
            ok(vm
                .package_ifneeded(&name.to_str(), &version)
                .map_or_else(Value::empty, Value::string))
        }
        [name, version, script] => {
            let version = version.to_str();
            if !validate_version_for(&version, vm.runtime_version()) {
                return invalid_version(&version);
            }
            vm.set_package_ifneeded(&name.to_str(), &version, &script.to_str());
            ok(Value::empty())
        }
        _ => err("wrong # args: should be \"package ifneeded package version ?script?\""),
    }
}

fn pkg_unknown(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [] => ok(vm
            .package_unknown()
            .map_or_else(Value::empty, Value::string)),
        [script] => {
            let script = script.to_str().to_string();
            vm.set_package_unknown((!script.is_empty()).then_some(script));
            ok(Value::empty())
        }
        _ => err("wrong # args: should be \"package unknown ?command?\""),
    }
}

/// `package prefer`'s preference word (`tclPkg.c`): C resolves it with
/// `Tcl_GetIndexFromObj(…, "preference", 0)`, so `l`/`s` abbreviate and the
/// empty word — a prefix of both entries — is `ambiguous preference ""`.
const PACKAGE_PREFERENCES: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("preference", &["latest", "stable"]);

fn pkg_prefer(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [] => ok(Value::string(preference_name(vm.package_prefer()))),
        [preference] => {
            let word = preference.to_str();
            match PACKAGE_PREFERENCES.index_of(word.as_bytes()) {
                Ok(0) => {
                    vm.prefer_latest_packages();
                    ok(Value::string(preference_name(vm.package_prefer())))
                }
                // The preference is a monotone Tcl latch: once raised to
                // latest, asking for stable succeeds but leaves it at latest.
                Ok(_) => ok(Value::string(preference_name(vm.package_prefer()))),
                Err(message) => err_with_code(
                    String::from_utf8_lossy(&message).into_owned(),
                    &tcl_syntax::list::join_list(["TCL", "LOOKUP", "INDEX", "preference", &word]),
                ),
            }
        }
        _ => err("wrong # args: should be \"package prefer ?preference?\""),
    }
}

fn preference_name(preference: PackagePrefer) -> &'static str {
    match preference {
        PackagePrefer::Stable => "stable",
        PackagePrefer::Latest => "latest",
    }
}

fn pkg_require(vm: &mut Vm, rest: &[Value], discover: bool) -> Completion<Value> {
    // `-exact NAME VERSION` is the requirement `VERSION-VERSION` — the same
    // rewrite `tclPkg.c`'s `PKG_REQUIRE` arm performs, so exactness needs no
    // second comparison rule.
    let exact = rest.first().is_some_and(|v| &*v.to_str() == "-exact");
    let rest = if exact { &rest[1..] } else { rest };
    let Some((name, reqs)) = rest.split_first() else {
        return err(
            "wrong # args: should be \"package require ?-exact? package ?requirement ...?\"",
        );
    };
    if exact && reqs.len() != 1 {
        return err(
            "wrong # args: should be \"package require ?-exact? package ?requirement ...?\"",
        );
    }
    let name = name.to_str();
    let release = vm.runtime_version();
    let requested: Vec<String> = reqs.iter().map(|r| r.to_str().to_string()).collect();
    if exact {
        if !validate_version_for(&requested[0], release) {
            return invalid_version(&requested[0]);
        }
    } else {
        for requirement in &requested {
            if let Err(error) = validate_requirement_for(requirement, release) {
                return invalid_requirement(error);
            }
        }
    }
    let reqs = requested.clone();
    match provided_status(vm, &name, &reqs, exact, release) {
        ProvidedStatus::Satisfies(version) => return ok(Value::string(version)),
        ProvidedStatus::Conflicts(version) => {
            return version_conflict(&name, &version, &requested, exact);
        }
        ProvidedStatus::Absent => {}
    }
    if discover && let Some(version) = vm.package_loading_version(&name) {
        return circular_dependency(&name, version, &requested, exact);
    }
    if !discover {
        return err(format!("package {name} is not present"));
    }

    // A loader already registered for a satisfying version wins before the
    // last-resort unknown callback. This is the ordinary fast path populated
    // by pkgIndex.tcl.
    if let Some(loader) = selected_loader(vm, &name, &reqs, exact, release) {
        return evaluate_loader(vm, &name, &loader);
    }

    if let Some(prefix) = vm.package_unknown().map(str::to_owned) {
        let mut callback = prefix;
        callback.push(' ');
        callback.push_str(&tcl_syntax::list::list_element(&name));
        let callback_requirements = if exact {
            vec![tcl_dialect::exact_requirement(&requested[0])]
        } else {
            reqs.clone()
        };
        for requirement in &callback_requirements {
            callback.push(' ');
            callback.push_str(&tcl_syntax::list::list_element(requirement));
        }
        let completion = eval_package_script(vm, &callback);
        if !completion.code.is_ok() {
            return completion;
        }
    }

    // The callback may provide the package directly or register a suitable
    // ifneeded script. Re-check both forms, in that order, before failing.
    match provided_status(vm, &name, &reqs, exact, release) {
        ProvidedStatus::Satisfies(version) => return ok(Value::string(version)),
        ProvidedStatus::Conflicts(version) => {
            return version_conflict(&name, &version, &requested, exact);
        }
        ProvidedStatus::Absent => {}
    }
    if let Some(loader) = selected_loader(vm, &name, &reqs, exact, release) {
        return evaluate_loader(vm, &name, &loader);
    }

    err(format!("can't find package {name}"))
}

enum ProvidedStatus {
    Absent,
    Satisfies(String),
    Conflicts(String),
}

fn provided_status(
    vm: &Vm,
    name: &str,
    requirements: &[String],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> ProvidedStatus {
    let Some(version) = vm.package_version(name).map(str::to_owned) else {
        return ProvidedStatus::Absent;
    };
    if requirements_satisfied(&version, requirements, exact, release) {
        ProvidedStatus::Satisfies(version)
    } else {
        ProvidedStatus::Conflicts(version)
    }
}

fn version_conflict(
    name: &str,
    have: &str,
    requested: &[String],
    exact: bool,
) -> Completion<Value> {
    let need = if exact {
        format!("exactly {}", requested[0])
    } else {
        requested.join(" ")
    };
    err_with_code(
        format!("version conflict for package \"{name}\": have {have}, need {need}"),
        "TCL PACKAGE VERSIONCONFLICT",
    )
}

fn invalid_version(version: &str) -> Completion<Value> {
    err_with_code(
        format!("expected version number but got \"{version}\""),
        "TCL VALUE VERSION",
    )
}

fn invalid_requirement(error: tcl_dialect::RequirementValidationError<'_>) -> Completion<Value> {
    match error {
        tcl_dialect::RequirementValidationError::InvalidVersion(version) => {
            invalid_version(version)
        }
        tcl_dialect::RequirementValidationError::InvalidRange(requirement) => err_with_code(
            format!("expected versionMin-versionMax but got \"{requirement}\""),
            "TCL VALUE VERSIONRANGE",
        ),
    }
}

fn circular_dependency(
    name: &str,
    loading_version: &str,
    requested: &[String],
    exact: bool,
) -> Completion<Value> {
    let required = if requested.is_empty() {
        name.to_owned()
    } else if exact {
        format!("{name} exactly {}", requested[0])
    } else {
        format!("{name} {}", requested.join(" "))
    };
    err_with_code(
        format!(
            "circular package dependency: attempt to provide {name} {loading_version} requires {required}"
        ),
        "TCL PACKAGE CIRCULARITY",
    )
}

struct SelectedLoader {
    version: String,
    script: String,
}

fn selected_loader(
    vm: &Vm,
    name: &str,
    requirements: &[String],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> Option<SelectedLoader> {
    let versions = vm.package_ifneeded_versions(name);
    let selected = if exact {
        select_package_version_exact_for(&versions, &requirements[0], release)?
    } else {
        let requirements: Vec<&str> = requirements.iter().map(String::as_str).collect();
        select_package_version_for(&versions, &requirements, vm.package_prefer(), release)?
    };
    let version = versions[selected].clone();
    vm.package_ifneeded(name, &version)
        .map(str::to_owned)
        .map(|script| SelectedLoader { version, script })
}

/// Package discovery scripts are interpreter-global even when the require was
/// issued in a proc or namespace. Keeping the frame transition here prevents
/// the unknown and ifneeded paths from drifting apart.
fn eval_package_script(vm: &mut Vm, script: &str) -> Completion<Value> {
    vm.eval_at_level(0, script)
}

fn evaluate_loader(vm: &mut Vm, name: &str, loader: &SelectedLoader) -> Completion<Value> {
    vm.begin_package_loading(name, &loader.version);
    let completion = eval_package_script(vm, &loader.script);
    vm.end_package_loading(name, &loader.version);
    if !completion.code.is_ok() {
        vm.forget_package(name);
        return completion;
    }
    match vm.package_version(name).map(str::to_owned) {
        Some(provided)
            if cmp_version(&provided, &loader.version, vm.runtime_version()) == Ordering::Equal =>
        {
            ok(Value::string(provided))
        }
        Some(provided) => {
            vm.forget_package(name);
            err_with_code(
                format!(
                    "attempt to provide package {name} {} failed: package {name} {provided} provided instead",
                    loader.version
                ),
                "TCL PACKAGE WRONGPROVIDE",
            )
        }
        None => err_with_code(
            format!(
                "attempt to provide package {name} {} failed: no version of package {name} provided",
                loader.version
            ),
            "TCL PACKAGE UNPROVIDED",
        ),
    }
}

fn requirements_satisfied(
    version: &str,
    requirements: &[String],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> bool {
    if requirements.is_empty() {
        true
    } else if exact {
        version_matches_exact_for(version, &requirements[0], release)
    } else {
        requirements
            .iter()
            .any(|requirement| vsatisfies(version, requirement, release))
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::{TclVersion, version_satisfies as vsatisfies};
    use tcl_runtime_api::Code;

    use crate::interp::Vm;
    use crate::value::Value;

    fn package(vm: &mut Vm, words: &[&str]) -> tcl_runtime_api::Completion<Value> {
        let args: Vec<Value> = words.iter().map(|word| Value::string(*word)).collect();
        super::cmd_package(vm, &args)
    }

    /// The pre-provided core packages follow the pinned release, so
    /// `package require Tcl 8.5` fails under a 9.x pin exactly as `tclsh9.0`
    /// fails it (`version conflict for package "Tcl": have 9.0.4, need 8.5`).
    /// Hardcoding `9.0.4` and providing `Tcl`+`tcl` regardless of the pin
    /// would make that require wrongly *succeed*.
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

    #[test]
    fn malformed_requirements_precede_an_active_loader_cycle() {
        let mut vm = Vm::new();
        vm.begin_package_loading("p", "1.0");
        let malformed = package(&mut vm, &["require", "p", "invalid"]);
        assert_eq!(malformed.code, Code::Error);
        assert_eq!(
            &*malformed.result.to_str(),
            "expected version number but got \"invalid\""
        );

        let cycle = package(&mut vm, &["require", "p", "1.0"]);
        assert_eq!(cycle.code, Code::Error);
        assert_eq!(
            &*cycle.result.to_str(),
            "circular package dependency: attempt to provide p 1.0 requires p 1.0"
        );
        vm.end_package_loading("p", "1.0");
    }

    #[test]
    fn package_commands_apply_the_pinned_plus_suffix_policy() {
        let mut tcl8 = Vm::new();
        tcl8.set_runtime_version(TclVersion::V8_6);
        let rejected = package(&mut tcl8, &["provide", "p", "1.2+platform"]);
        assert_eq!(rejected.code, Code::Error);
        assert_eq!(
            &*rejected.result.to_str(),
            "expected version number but got \"1.2+platform\""
        );
        assert_eq!(tcl8.package_version("p"), None);
        let rejected = package(&mut tcl8, &["ifneeded", "p", "1.2+platform", ""]);
        assert_eq!(rejected.code, Code::Error);
        assert!(tcl8.package_ifneeded_versions("p").is_empty());

        let mut tcl9 = Vm::new();
        tcl9.set_runtime_version(TclVersion::V9_0);
        let provided = package(&mut tcl9, &["provide", "p", "1.2+platform"]);
        assert_eq!(provided.code, Code::Ok);
        assert_eq!(tcl9.package_version("p"), Some("1.2+platform"));
        let compared = package(&mut tcl9, &["vcompare", "1.2+platform", "1.2"]);
        assert_eq!(compared.code, Code::Ok);
        assert_eq!(&*compared.result.to_str(), "0");
        let registered = package(&mut tcl9, &["ifneeded", "q", "1.2+platform", ""]);
        assert_eq!(registered.code, Code::Ok);
        assert_eq!(tcl9.package_ifneeded_versions("q"), vec!["1.2+platform"]);

        let mut exact_provider = Vm::new();
        exact_provider.set_runtime_version(TclVersion::V9_0);
        assert_eq!(
            package(&mut exact_provider, &["ifneeded", "q", "1.2+platform", ""]).code,
            Code::Ok
        );
        assert_eq!(
            package(&mut exact_provider, &["ifneeded", "q", "1.3", ""]).code,
            Code::Ok
        );
        let exact_loader = super::selected_loader(
            &exact_provider,
            "q",
            &["1.2+platform".to_owned()],
            true,
            TclVersion::V9_0,
        )
        .expect("exact Tcl 9 provider selection");
        assert_eq!(exact_loader.version, "1.2+platform");

        let mut exact = Vm::new();
        exact.set_runtime_version(TclVersion::V9_0);
        assert_eq!(package(&mut exact, &["provide", "p", "1.3"]).code, Code::Ok);
        let mismatch = package(&mut exact, &["require", "-exact", "p", "1.2+x"]);
        assert_eq!(mismatch.code, Code::Error);
        assert_eq!(
            &*mismatch.result.to_str(),
            "version conflict for package \"p\": have 1.3, need exactly 1.2+x"
        );
        let match_result = package(&mut exact, &["require", "-exact", "p", "1.3+x"]);
        assert_eq!(match_result.code, Code::Ok);
        assert_eq!(&*match_result.result.to_str(), "1.3");

        // A profile change does not rewrite package state. Lookup and
        // provider selection must nevertheless apply the new release's
        // grammar, so a Tcl 9-only provider is not selected by an 8.6 pin.
        tcl9.set_runtime_version(TclVersion::V8_6);
        assert!(super::selected_loader(&tcl9, "q", &[], false, TclVersion::V8_6).is_none());
        let conflict = package(&mut tcl9, &["require", "p", "1.2"]);
        assert_eq!(conflict.code, Code::Error);
        assert_eq!(tcl9.package_version("p"), Some("1.2+platform"));
    }
}
