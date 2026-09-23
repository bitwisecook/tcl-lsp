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

//! `package` — the package database (toward running tcltest). C ref `tclPkg.c`.
//!
//! Holds the provided-version map, the `ifneeded` load scripts, and the
//! `unknown` handler. `require` returns a provided package's version (checking
//! the requirements), else invokes the `unknown` handler (the pure-Tcl
//! auto-loader, which needs the VFS — L2) and re-checks. Version requirements
//! follow TIP 268 (`min-`, `min-max`, bare `min` = same major), verified vs
//! tclsh 9.0. The core `tcl`/`Tcl` packages are pre-provided at interp start
//! (as C does before sourcing `init.tcl`).

use std::collections::BTreeMap;

use tcl_dialect::{
    compare_versions_for, exact_requirement, select_package_version_exact_for,
    select_package_version_for, validate_requirement_for, validate_version_for,
    version_matches_exact_for, version_satisfies_for, PackagePrefer, TclVersion,
};

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// The interpreter's package database.
#[derive(Default)]
pub struct PackageState {
    /// `package provide` — package name → version.
    provided: BTreeMap<Vec<u8>, Vec<u8>>,
    /// `package ifneeded` — (name, version) → load script.
    ifneeded: BTreeMap<(Vec<u8>, Vec<u8>), Vec<u8>>,
    /// `package unknown` — the handler command prefix.
    unknown: Vec<u8>,
}

impl PackageState {
    /// A fresh database with `version`'s core packages pre-provided (what C
    /// registers before `init.tcl` runs).
    #[must_use]
    pub fn with_core(version: TclVersion) -> PackageState {
        let mut p = PackageState::default();
        p.provide_core(version);
        p
    }

    /// Install the core packages a bare interpreter of `version` pre-provides,
    /// withdrawing whatever a previous pin left behind.
    ///
    /// Which names exist and what version each carries is release data
    /// ([`TclVersion::core_provided_packages`]), not a runtime constant:
    /// `tclsh8.6` provides `Tcl` and `TclOO 1.1.0` and neither lowercase
    /// spelling, while `tclsh9.0` provides all four at `9.0.4`/`1.3.1`, and
    /// `tclsh8.4` provides only `Tcl` at the two-component `8.4`. Freezing
    /// that at 9.0's answer made `package require Tcl 8.5` succeed under an
    /// 8.x pin, where real `tclsh8.4` raises a version conflict (ledger
    /// row B4).
    ///
    /// C also registers `ifneeded` entries for the `TclOO` names at their
    /// provided version (its `initScript`), so they show up in
    /// `package versions` (oo-0.9).
    pub(crate) fn provide_core(&mut self, version: TclVersion) {
        // Derived from the same table rather than hand-listed, so a name added
        // to one release's row cannot be left behind by a re-pin.
        for stale in TclVersion::ALL {
            for core in stale.core_provided_packages() {
                self.provided.remove(core.name.as_bytes());
                self.ifneeded
                    .retain(|(name, _), _| name != core.name.as_bytes());
            }
        }
        for core in version.core_provided_packages() {
            let (name, provided) = (core.name.as_bytes(), core.version.as_bytes());
            self.provided.insert(name.to_vec(), provided.to_vec());
            if core.ifneeded_stub {
                self.ifneeded.insert(
                    (name.to_vec(), provided.to_vec()),
                    b"# Already present, OK?".to_vec(),
                );
            }
        }
    }
}

/// Register `package`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"package", package_cmd);
}

/// `package`'s subcommand words, in C table order (`pkgOptions[]`,
/// `tclPkg.c`). C resolves them with `Tcl_GetIndexFromObj(…, "option", 0)`,
/// so `n` resolves to `names` while `v` and `pr` are ambiguous, and the empty
/// word — a prefix of every entry — is `ambiguous option ""`.
///
/// As with `interp`, the table names only what this runtime
/// dispatches: C also carries `files` (9.0), `forget`, and `prefer`, which
/// need a package loader and a preference latch this runtime has none of.
/// tclsh 9.0.4, for contrast:
///   package x -> bad option "x": must be files, forget, ifneeded, names,
///                prefer, present, provide, require, unknown, vcompare,
///                versions, or vsatisfies
const PACKAGE_OPTIONS: &[&[u8]] = &[
    b"ifneeded",
    b"names",
    b"present",
    b"provide",
    b"require",
    b"unknown",
    b"vcompare",
    b"versions",
    b"vsatisfies",
];

fn package_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"package subcommand ?arg ...?");
    }
    let word = obj_bytes(argv[1]);
    let sub = match tcl_cmd_core::prefix::OptionTable::abbreviating("option", PACKAGE_OPTIONS)
        .index_of(&word)
    {
        Ok(i) => PACKAGE_OPTIONS[i],
        Err(message) => {
            let code =
                crate::interp::error_code_list(&[b"TCL", b"LOOKUP", b"INDEX", b"option", &word]);
            return interp.error_with_code(&message, &code);
        }
    };
    match sub {
        b"provide" => provide(interp, argv),
        b"require" => require(interp, argv),
        b"present" => present(interp, argv),
        b"ifneeded" => ifneeded(interp, argv),
        b"unknown" => unknown(interp, argv),
        b"names" => names(interp, argv),
        b"versions" => versions(interp, argv),
        b"vsatisfies" => vsatisfies_cmd(interp, argv),
        b"vcompare" => vcompare_cmd(interp, argv),
        // Unreachable: every name in `PACKAGE_OPTIONS` has an arm above.
        other => {
            let mut m = b"bad option \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be ");
            m.extend_from_slice(&tcl_cmd_core::prefix::choice_list_bytes(PACKAGE_OPTIONS));
            interp.set_error(&m)
        }
    }
}

/// `package provide name ?version?`.
fn provide(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        3 => {
            let name = obj_bytes(argv[2]);
            let v = interp
                .packages
                .borrow()
                .provided
                .get(&name)
                .cloned()
                .unwrap_or_default();
            interp.set_result_bytes(&v);
            Code::Ok
        }
        4 => {
            let name = obj_bytes(argv[2]);
            let version = obj_bytes(argv[3]);
            if !valid_version(&version, interp.runtime_version()) {
                return invalid_version(interp, &version);
            }
            let existing = interp.packages.borrow().provided.get(&name).cloned();
            if let Some(existing) = existing {
                if compare_versions(&existing, &version, interp.runtime_version())
                    != core::cmp::Ordering::Equal
                {
                    let mut message = b"conflicting versions provided for package \"".to_vec();
                    message.extend_from_slice(&name);
                    message.extend_from_slice(b"\": ");
                    message.extend_from_slice(&existing);
                    message.extend_from_slice(b", then ");
                    message.extend_from_slice(&version);
                    return interp.error_with_code(&message, b"TCL PACKAGE VERSIONCONFLICT");
                }
                interp.set_result_bytes(b"");
                return Code::Ok;
            }
            interp.packages.borrow_mut().provided.insert(name, version);
            interp.set_result_bytes(b"");
            Code::Ok
        }
        _ => interp.wrong_args(b"package provide name ?version?"),
    }
}

/// `package require ?-exact? name ?requirement ...?`.
fn require(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    package_lookup(interp, argv, true)
}

/// `package present ?-exact? name ?requirement ...?` — like require, no loading.
fn present(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    package_lookup(interp, argv, false)
}

fn package_lookup(interp: &mut Interp, argv: &[*mut TclObj], load: bool) -> Code {
    let mut index = 2;
    let exact = argv
        .get(index)
        .is_some_and(|&arg| obj_bytes(arg).as_slice() == b"-exact");
    if exact {
        index += 1;
    }
    let Some(&name_obj) = argv.get(index) else {
        return interp.wrong_args(if load {
            b"package require ?-exact? package ?requirement ...?"
        } else {
            b"package present ?-exact? package ?requirement ...?"
        });
    };
    let requirements: Vec<Vec<u8>> = argv[index + 1..]
        .iter()
        .map(|&arg| obj_bytes(arg))
        .collect();
    if exact && requirements.len() != 1 {
        return interp.wrong_args(if load {
            b"package require ?-exact? package ?requirement ...?"
        } else {
            b"package present ?-exact? package ?requirement ...?"
        });
    }
    let release = interp.runtime_version();
    if exact {
        if !valid_version(&requirements[0], release) {
            return invalid_version(interp, &requirements[0]);
        }
    } else {
        for requirement in &requirements {
            if let Some(error) = invalid_requirement_kind(requirement, release) {
                return invalid_requirement(interp, error);
            }
        }
    }
    let name = obj_bytes(name_obj);
    match check_provided(interp, &name, exact, &requirements) {
        Some(Ok(version)) => {
            interp.set_result_bytes(&version);
            return Code::Ok;
        }
        Some(Err(code)) => return code,
        None if !load => {
            let mut message = b"package ".to_vec();
            message.extend_from_slice(&name);
            message.extend_from_slice(b" is not present");
            return interp.set_error(&message);
        }
        None => {}
    }
    for attempt in 0..2 {
        if let Some(version) = best_ifneeded(interp, &name, exact, &requirements) {
            let script = interp.packages.borrow().ifneeded[&(name.clone(), version)].clone();
            if interp.eval_uplevel(0, &script) == Code::Error {
                return Code::Error;
            }
            if let Some(Ok(version)) = check_provided(interp, &name, exact, &requirements) {
                interp.set_result_bytes(&version);
                return Code::Ok;
            }
        }
        if attempt == 0 {
            let handler = interp.packages.borrow().unknown.clone();
            if handler.is_empty() {
                break;
            }
            let mut script = handler;
            script.push(b' ');
            tcl_syntax::list::append_list_element(&mut script, &name, false);
            for requirement in &requirements {
                script.push(b' ');
                if exact {
                    let exact_requirement = exact_requirement(&version_text(requirement));
                    tcl_syntax::list::append_list_element(
                        &mut script,
                        exact_requirement.as_bytes(),
                        false,
                    );
                } else {
                    tcl_syntax::list::append_list_element(&mut script, requirement, false);
                }
            }
            if interp.eval_uplevel(0, &script) == Code::Error {
                return Code::Error;
            }
        }
    }
    let mut message = b"can't find package ".to_vec();
    message.extend_from_slice(&name);
    interp.set_error(&message)
}

/// The highest `ifneeded` version of `name` satisfying the requirements (for
/// `-exact`, an equivalent version), or `None`.
fn best_ifneeded(
    interp: &Interp,
    name: &[u8],
    exact: bool,
    requirements: &[Vec<u8>],
) -> Option<Vec<u8>> {
    let candidates: Vec<Vec<u8>> = interp
        .packages
        .borrow()
        .ifneeded
        .keys()
        .filter(|(candidate_name, _)| candidate_name.as_slice() == name)
        .map(|(_, version)| version.clone())
        .collect();
    let versions: Vec<String> = candidates
        .iter()
        .map(|version| version_text(version))
        .collect();
    let release = interp.runtime_version();
    let selected = if exact {
        select_package_version_exact_for(&versions, &version_text(&requirements[0]), release)
    } else {
        let requirements: Vec<String> = requirements
            .iter()
            .map(|requirement| version_text(requirement))
            .collect();
        let requirements: Vec<&str> = requirements.iter().map(String::as_str).collect();
        // `package prefer` is not implemented here, so preserve Tcl's initial
        // preference: stable providers win when one satisfies the request.
        select_package_version_for(&versions, &requirements, PackagePrefer::Stable, release)
    }?;
    Some(candidates[selected].clone())
}

/// If `name` is provided and satisfies the requirements, `Some(Ok(version))`; if
/// provided but unsatisfactory, `Some(Err(code))`; if not provided, `None`.
fn check_provided(
    interp: &mut Interp,
    name: &[u8],
    exact: bool,
    requirements: &[Vec<u8>],
) -> Option<Result<Vec<u8>, Code>> {
    let version = interp.packages.borrow().provided.get(name)?.clone();
    let release = interp.runtime_version();
    let satisfied = requirements.is_empty()
        || if exact {
            version_matches_exact_for(
                &version_text(&version),
                &version_text(&requirements[0]),
                release,
            )
        } else {
            requirements.iter().any(|requirement| {
                version_satisfies_for(&version_text(&version), &version_text(requirement), release)
            })
        };
    if satisfied {
        Some(Ok(version))
    } else {
        let mut message = b"version conflict for package \"".to_vec();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\": have ");
        message.extend_from_slice(&version);
        message.extend_from_slice(b", need ");
        if exact {
            message.extend_from_slice(b"exactly ");
        }
        for (index, requirement) in requirements.iter().enumerate() {
            if index != 0 {
                message.push(b' ');
            }
            message.extend_from_slice(requirement);
        }
        Some(Err(
            interp.error_with_code(&message, b"TCL PACKAGE VERSIONCONFLICT")
        ))
    }
}

/// `package ifneeded name version ?script?`.
fn ifneeded(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 || argv.len() > 5 {
        return interp.wrong_args(b"package ifneeded package version ?script?");
    }
    let key = (obj_bytes(argv[2]), obj_bytes(argv[3]));
    if !valid_version(&key.1, interp.runtime_version()) {
        return invalid_version(interp, &key.1);
    }
    if argv.len() == 5 {
        interp
            .packages
            .borrow_mut()
            .ifneeded
            .insert(key, obj_bytes(argv[4]));
        interp.set_result_bytes(b"");
    } else {
        let script = interp
            .packages
            .borrow()
            .ifneeded
            .get(&key)
            .cloned()
            .unwrap_or_default();
        interp.set_result_bytes(&script);
    }
    Code::Ok
}

/// `package unknown ?command?`.
fn unknown(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        2 => {
            let u = interp.packages.borrow().unknown.clone();
            interp.set_result_bytes(&u);
            Code::Ok
        }
        3 => {
            interp.packages.borrow_mut().unknown = obj_bytes(argv[2]);
            interp.set_result_bytes(b"");
            Code::Ok
        }
        _ => interp.wrong_args(b"package unknown ?command?"),
    }
}

/// `package names` — sorted names of all provided / ifneeded packages.
fn names(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"package names");
    }
    let mut set: std::collections::BTreeSet<Vec<u8>> =
        interp.packages.borrow().provided.keys().cloned().collect();
    for (n, _) in interp.packages.borrow().ifneeded.keys() {
        set.insert(n.clone());
    }
    let objs: Vec<*mut TclObj> = set.iter().map(|n| crate::interp::new_string(n)).collect();
    interp.set_result(crate::list::new_list_obj(&objs));
    Code::Ok
}

/// `package versions name` — versions with an `ifneeded` script.
fn versions(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"package versions package");
    }
    let name = obj_bytes(argv[2]);
    let vers: Vec<*mut TclObj> = interp
        .packages
        .borrow()
        .ifneeded
        .keys()
        .filter(|(n, _)| *n == name)
        .map(|(_, v)| crate::interp::new_string(v))
        .collect();
    interp.set_result(crate::list::new_list_obj(&vers));
    Code::Ok
}

fn vsatisfies_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"package vsatisfies version requirement ?requirement ...?");
    }
    let version = obj_bytes(argv[2]);
    let release = interp.runtime_version();
    if !valid_version(&version, release) {
        return invalid_version(interp, &version);
    }
    for &requirement in &argv[3..] {
        let requirement = obj_bytes(requirement);
        if let Some(error) = invalid_requirement_kind(&requirement, release) {
            return invalid_requirement(interp, error);
        }
    }
    let satisfied = argv[3..].iter().any(|&requirement| {
        version_satisfies_for(
            &version_text(&version),
            &version_text(&obj_bytes(requirement)),
            release,
        )
    });
    interp.set_result_bytes(if satisfied { b"1" } else { b"0" });
    Code::Ok
}

fn vcompare_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"package vcompare version1 version2");
    }
    let left = obj_bytes(argv[2]);
    let right = obj_bytes(argv[3]);
    let release = interp.runtime_version();
    if !valid_version(&left, release) {
        return invalid_version(interp, &left);
    }
    if !valid_version(&right, release) {
        return invalid_version(interp, &right);
    }
    let comparison = compare_versions(&left, &right, release);
    interp.set_result_bytes(match comparison {
        core::cmp::Ordering::Less => b"-1",
        core::cmp::Ordering::Equal => b"0",
        core::cmp::Ordering::Greater => b"1",
    });
    Code::Ok
}

fn version_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn valid_version(bytes: &[u8], release: TclVersion) -> bool {
    validate_version_for(&version_text(bytes), release)
}

enum InvalidRequirement {
    Version(Vec<u8>),
    Range(Vec<u8>),
}

fn invalid_requirement_kind(bytes: &[u8], release: TclVersion) -> Option<InvalidRequirement> {
    match validate_requirement_for(&version_text(bytes), release) {
        Ok(()) => None,
        Err(tcl_dialect::RequirementValidationError::InvalidVersion(version)) => {
            Some(InvalidRequirement::Version(version.as_bytes().to_vec()))
        }
        Err(tcl_dialect::RequirementValidationError::InvalidRange(requirement)) => {
            Some(InvalidRequirement::Range(requirement.as_bytes().to_vec()))
        }
    }
}

fn compare_versions(left: &[u8], right: &[u8], release: TclVersion) -> core::cmp::Ordering {
    compare_versions_for(&version_text(left), &version_text(right), release)
}

fn invalid_version(interp: &mut Interp, version: &[u8]) -> Code {
    let mut message = b"expected version number but got \"".to_vec();
    message.extend_from_slice(version);
    message.push(b'"');
    interp.error_with_code(&message, b"TCL VALUE VERSION")
}

fn invalid_requirement(interp: &mut Interp, error: InvalidRequirement) -> Code {
    match error {
        InvalidRequirement::Version(version) => invalid_version(interp, &version),
        InvalidRequirement::Range(requirement) => {
            let mut message = b"expected versionMin-versionMax but got \"".to_vec();
            message.extend_from_slice(&requirement);
            message.push(b'"');
            interp.error_with_code(&message, b"TCL VALUE VERSIONRANGE")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(counters::finalize(), 0, "leak");
        assert_eq!(counters::double_free_count(), 0);
    }

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    /// `package`'s subcommand word is a `Tcl_GetIndexFromObj(…,
    /// "option", 0)` table (`pkgOptions[]`, `tclPkg.c`), not an ensemble, so
    /// the miss sentence is `bad option`/`ambiguous option`, never an
    /// ensemble's `unknown or ambiguous subcommand` wording, and words
    /// abbreviate by unique prefix. The list still
    /// names only what this runtime dispatches (`files`, `forget`, and
    /// `prefer` need a loader and a preference latch it has none of).
    ///
    /// tclsh 8.6.16 / 9.0.4 (the verdicts, not the shortened list):
    ///   package x  -> bad option "x": must be …    [TCL LOOKUP INDEX option x]
    ///   package {} -> ambiguous option "": must be …
    ///   package v  -> ambiguous option "v": must be …  (vcompare/versions/vsatisfies)
    ///   package n  -> the names list
    #[test]
    fn package_option_word_resolves_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be ifneeded, names, present, provide, require, unknown, \
                            vcompare, versions, or vsatisfies";
        leak_free(|i| {
            let err = |i: &mut Interp, script: &[u8]| {
                assert_eq!(i.eval_str(script), Code::Error, "expected an error");
                String::from_utf8_lossy(&i.result_bytes()).into_owned()
            };
            assert_eq!(err(i, b"package x"), format!("bad option \"x\": {MUST}"));
            assert_eq!(
                err(i, b"package {}"),
                format!("ambiguous option \"\": {MUST}")
            );
            assert_eq!(
                err(i, b"package v"),
                format!("ambiguous option \"v\": {MUST}")
            );
            // A unique prefix resolves.
            assert_eq!(run(i, b"package provide foo 1.0"), b"");
            assert_eq!(
                run(i, b"llength [lsearch -all -exact [package n] foo]"),
                b"1"
            );
            // C's lookup error code travels with the message.
            assert_eq!(
                run(i, b"catch {package x} e opts; dict get $opts -errorcode"),
                b"TCL LOOKUP INDEX option x"
            );
        });
    }

    #[test]
    fn core_require_and_provide() {
        leak_free(|i| {
            assert_eq!(run(i, b"package require -exact tcl 9.0.4"), b"9.0.4");
            assert_eq!(run(i, b"package require Tcl 8.5-"), b"9.0.4");
            assert_eq!(run(i, b"package provide Tcl"), b"9.0.4");
            run(i, b"package provide mypkg 1.2");
            assert_eq!(run(i, b"package require mypkg"), b"1.2");
            assert_eq!(i.eval_str(b"package require nosuch"), Code::Error);
        });
    }

    #[test]
    fn vsatisfies_and_vcompare() {
        leak_free(|i| {
            assert_eq!(run(i, b"package vsatisfies 9.0.4 9.0-"), b"1");
            assert_eq!(run(i, b"package vsatisfies 9.0.4 8.5-9.0"), b"0");
            assert_eq!(run(i, b"package vsatisfies 8.6.1 8.5"), b"1");
            assert_eq!(run(i, b"package vsatisfies 9.0 8.5"), b"0");
            assert_eq!(run(i, b"package vcompare 8.5 9.0"), b"-1");
            assert_eq!(run(i, b"package vcompare 9.0.4 9.0.4"), b"0");
        });
    }

    #[test]
    fn package_versions_use_the_pinned_shared_release_policy() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            for command in [
                "package provide p 1.2+x",
                "package ifneeded p 1.2+x {}",
                "package vsatisfies 1.2 1.2+x",
                "package vcompare 1.2+x 1.2",
                "package require absent 1.2+x",
                "package present absent 1.2+x",
            ] {
                assert_eq!(i.eval_str(command.as_bytes()), Code::Error, "{command}");
                let caught =
                    format!("catch {{{command}}} message options; dict get $options -errorcode");
                assert_eq!(run(i, caught.as_bytes()), b"TCL VALUE VERSION", "{command}");
            }
            assert_eq!(run(i, b"package provide p 1.2"), b"");
            assert_eq!(
                run(i, b"catch {package provide p 1.3} message; set message"),
                b"conflicting versions provided for package \"p\": 1.2, then 1.3"
            );
            assert_eq!(
                run(
                    i,
                    b"catch {package provide p 1.3} message options; dict get $options -errorcode"
                ),
                b"TCL PACKAGE VERSIONCONFLICT"
            );

            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            assert_eq!(
                run(
                    i,
                    b"catch {package vsatisfies 1 1-bad} message; set message"
                ),
                b"expected version number but got \"bad\""
            );
            assert_eq!(
                run(i, b"catch {package vsatisfies 1 1-2-3} message options; dict get $options -errorcode"),
                b"TCL VALUE VERSIONRANGE"
            );
            assert_eq!(run(i, b"package provide q 1.2+x"), b"");
            assert_eq!(run(i, b"package provide q 1.2+y"), b"");
            assert_eq!(run(i, b"package vcompare 1.2+x 1.2"), b"0");
            assert_eq!(
                run(i, b"package ifneeded r 1.2+x {package provide r 1.2+x}"),
                b""
            );
            assert_eq!(run(i, b"package require -exact r 1.2+y"), b"1.2+x");
            run(
                i,
                b"proc package_callback args {set ::package_callback $args}",
            );
            run(i, b"package unknown package_callback");
            assert_eq!(
                run(
                    i,
                    b"catch {package require -exact absent {1.2+x space{brace}}}; set ::package_callback"
                ),
                b"absent {1.2+x space{brace}-1.2+x space{brace}}"
            );
        });
    }

    #[test]
    fn ifneeded_and_unknown() {
        leak_free(|i| {
            run(i, b"package ifneeded foo 1.0 {set loaded yes}");
            assert_eq!(run(i, b"package ifneeded foo 1.0"), b"set loaded yes");
            run(i, b"package unknown myhandler");
            assert_eq!(run(i, b"package unknown"), b"myhandler");
        });
    }
}
