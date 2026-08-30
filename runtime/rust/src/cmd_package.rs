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

use tcl_dialect::TclVersion;

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

fn package_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"package subcommand ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        b"provide" => provide(interp, argv),
        b"require" => require(interp, argv),
        b"present" => present(interp, argv),
        b"ifneeded" => ifneeded(interp, argv),
        b"unknown" => unknown(interp, argv),
        b"names" => names(interp, argv),
        b"versions" => versions(interp, argv),
        b"vsatisfies" => vsatisfies_cmd(interp, argv),
        b"vcompare" => vcompare_cmd(interp, argv),
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be ifneeded, names, present, provide, require, unknown, vcompare, versions, or vsatisfies");
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
            let existing = interp.packages.borrow().provided.get(&name).cloned();
            if let Some(existing) = existing {
                if existing != version {
                    let mut m = b"conflicting versions provided for package \"".to_vec();
                    m.extend_from_slice(&name);
                    m.extend_from_slice(b"\"");
                    return interp.set_error(&m);
                }
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
    let mut i = 2;
    let exact = argv
        .get(i)
        .is_some_and(|&a| obj_bytes(a).as_slice() == b"-exact");
    if exact {
        i += 1;
    }
    let Some(&name_obj) = argv.get(i) else {
        return interp.wrong_args(b"package require ?-exact? package ?requirement ...?");
    };
    let name = obj_bytes(name_obj);
    let reqs: Vec<Vec<u8>> = argv[i + 1..].iter().map(|&a| obj_bytes(a)).collect();

    // 1. Already provided and satisfactory?
    if let Some(found) = check_provided(interp, &name, exact, &reqs) {
        return match found {
            Ok(v) => {
                interp.set_result_bytes(&v);
                Code::Ok
            }
            Err(code) => code,
        };
    }
    // 2. Try an `ifneeded` load script for a satisfying version; if none yet,
    //    run the `unknown` handler (auto-loader / pkgIndex search) and retry.
    for attempt in 0..2 {
        if let Some(ver) = best_ifneeded(interp, &name, exact, &reqs) {
            let script = interp.packages.borrow().ifneeded[&(name.clone(), ver)].clone();
            // Package load scripts run at the global scope (C's `uplevel #0`),
            // so a package's `namespace eval foo` creates `::foo` even when
            // `package require` is called from inside another namespace.
            if interp.eval_uplevel(0, &script) == Code::Error {
                return Code::Error;
            }
            if let Some(Ok(v)) = check_provided(interp, &name, exact, &reqs) {
                interp.set_result_bytes(&v);
                return Code::Ok;
            }
        }
        if attempt == 0 {
            let handler = interp.packages.borrow().unknown.clone();
            if handler.is_empty() {
                break;
            }
            let mut script = handler;
            for w in std::iter::once(&name).chain(reqs.iter()) {
                script.push(b' ');
                script.extend_from_slice(w);
            }
            // The `package unknown` handler (pkgIndex search) also runs globally.
            if interp.eval_uplevel(0, &script) == Code::Error {
                return Code::Error;
            }
        }
    }
    let mut m = b"can't find package ".to_vec();
    m.extend_from_slice(&name);
    interp.set_error(&m)
}

/// The highest `ifneeded` version of `name` satisfying the requirements (for
/// `-exact`, an exact match), or `None`.
fn best_ifneeded(interp: &Interp, name: &[u8], exact: bool, reqs: &[Vec<u8>]) -> Option<Vec<u8>> {
    let mut best: Option<Vec<u8>> = None;
    for (n, v) in interp.packages.borrow().ifneeded.keys() {
        if n != name {
            continue;
        }
        let ok = if exact {
            reqs.first().is_none_or(|r| r == v)
        } else {
            reqs.iter().all(|r| vsatisfies(v, r))
        };
        if ok
            && best
                .as_ref()
                .is_none_or(|b| vcompare(v, b) == core::cmp::Ordering::Greater)
        {
            best = Some(v.clone());
        }
    }
    best
}

/// If `name` is provided and satisfies the requirements, `Some(Ok(version))`; if
/// provided but unsatisfactory, `Some(Err(code))`; if not provided, `None`.
fn check_provided(
    interp: &mut Interp,
    name: &[u8],
    exact: bool,
    reqs: &[Vec<u8>],
) -> Option<Result<Vec<u8>, Code>> {
    let v = interp.packages.borrow().provided.get(name)?.clone();
    let ok = if exact {
        reqs.first().is_none_or(|r| r == &v)
    } else {
        reqs.iter().all(|r| vsatisfies(&v, r))
    };
    if ok {
        Some(Ok(v))
    } else {
        let mut m = b"version conflict for package \"".to_vec();
        m.extend_from_slice(name);
        m.extend_from_slice(b"\": have ");
        m.extend_from_slice(&v);
        Some(Err(interp.set_error(&m)))
    }
}

/// `package present ?-exact? name ?requirement ...?` — like require, no loading.
fn present(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut i = 2;
    let exact = argv
        .get(i)
        .is_some_and(|&a| obj_bytes(a).as_slice() == b"-exact");
    if exact {
        i += 1;
    }
    let Some(&name_obj) = argv.get(i) else {
        return interp.wrong_args(b"package present ?-exact? package ?requirement ...?");
    };
    let name = obj_bytes(name_obj);
    let reqs: Vec<Vec<u8>> = argv[i + 1..].iter().map(|&a| obj_bytes(a)).collect();
    match check_provided(interp, &name, exact, &reqs) {
        Some(Ok(v)) => {
            interp.set_result_bytes(&v);
            Code::Ok
        }
        Some(Err(code)) => code,
        None => {
            let mut m = b"package ".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b" is not present");
            interp.set_error(&m)
        }
    }
}

/// `package ifneeded name version ?script?`.
fn ifneeded(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 || argv.len() > 5 {
        return interp.wrong_args(b"package ifneeded package version ?script?");
    }
    let key = (obj_bytes(argv[2]), obj_bytes(argv[3]));
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
    let v = obj_bytes(argv[2]);
    let ok = argv[3..].iter().all(|&r| vsatisfies(&v, &obj_bytes(r)));
    interp.set_result_bytes(if ok { b"1" } else { b"0" });
    Code::Ok
}

fn vcompare_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"package vcompare version1 version2");
    }
    let c = vcompare(&obj_bytes(argv[2]), &obj_bytes(argv[3]));
    interp.set_result_bytes(format!("{}", c as i32).as_bytes());
    Code::Ok
}

// -- version arithmetic (TIP 268) ------------------------------------------

fn components(v: &[u8]) -> Vec<i64> {
    v.split(|&b| b == b'.')
        .map(|p| {
            core::str::from_utf8(p)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        })
        .collect()
}

/// Compare two dot-separated integer versions.
fn vcompare(a: &[u8], b: &[u8]) -> core::cmp::Ordering {
    let (ca, cb) = (components(a), components(b));
    let n = ca.len().max(cb.len());
    for i in 0..n {
        let x = ca.get(i).copied().unwrap_or(0);
        let y = cb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
    }
    core::cmp::Ordering::Equal
}

/// Whether version `v` satisfies a single requirement (`min-`, `min-max`, or a
/// bare `min` meaning the same major series).
fn vsatisfies(v: &[u8], req: &[u8]) -> bool {
    use core::cmp::Ordering::Less;
    if let Some(dash) = req.iter().position(|&b| b == b'-') {
        let min = &req[..dash];
        let max = &req[dash + 1..];
        if vcompare(v, min) == Less {
            return false; // v < min
        }
        if max.is_empty() {
            true // `min-` : unbounded above
        } else {
            vcompare(v, max) == Less // `min-max` : v < max
        }
    } else {
        // Bare `min`: v >= min and v < (firstComponent(min) + 1).
        if vcompare(v, req) == Less {
            return false;
        }
        let bump = components(req).first().copied().unwrap_or(0) + 1;
        vcompare(v, bump.to_string().as_bytes()) == Less
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
    fn ifneeded_and_unknown() {
        leak_free(|i| {
            run(i, b"package ifneeded foo 1.0 {set loaded yes}");
            assert_eq!(run(i, b"package ifneeded foo 1.0"), b"set loaded yes");
            run(i, b"package unknown myhandler");
            assert_eq!(run(i, b"package unknown"), b"myhandler");
        });
    }
}
