//! The `package` command — a minimal version registry.
//!
//! Enough to satisfy the `package require Tcl …` / `package provide` /
//! `package vsatisfies` calls at the top of library scripts like `tcltest`.
//! No real on-disk package loading (`ifneeded` scripts) is performed yet.

use std::cmp::Ordering;

use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    // We emulate Tcl 9.0; advertise it so `package require Tcl` resolves.
    vm.provide_package("Tcl", "9.0");
    vm.register("package", cmd_package);
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
        "names" => {
            let mut names = vm.package_names();
            names.sort();
            ok(Value::list(names.into_iter().map(Value::string).collect()))
        }
        // Accepted no-ops.
        "ifneeded" | "forget" | "unknown" | "prefer" => ok(Value::empty()),
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
    }
}

fn pkg_require(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    // Skip a leading `-exact` flag.
    let rest = match rest.first() {
        Some(v) if &*v.to_str() == "-exact" => &rest[1..],
        _ => rest,
    };
    let Some((name, reqs)) = rest.split_first() else {
        return err(
            "wrong # args: should be \"package require ?-exact? package ?requirement ...?\"",
        );
    };
    let name = name.to_str();
    match vm.package_version(&name) {
        Some(v) => {
            let v = v.to_string();
            let ok_version = reqs.is_empty() || reqs.iter().any(|r| vsatisfies(&v, &r.to_str()));
            if ok_version {
                ok(Value::string(v))
            } else {
                err(format!("version conflict for package \"{name}\": have {v}"))
            }
        }
        None => err(format!("can't find package {name}")),
    }
}

/// Parse a dotted version into numeric components.
fn parse_version(v: &str) -> Vec<u64> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// Compare two dotted versions component-wise (missing components are 0).
fn cmp_version(a: &str, b: &str) -> Ordering {
    let (va, vb) = (parse_version(a), parse_version(b));
    for i in 0..va.len().max(vb.len()) {
        let x = va.get(i).copied().unwrap_or(0);
        let y = vb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

/// Does `version` satisfy a Tcl version requirement?
///
/// - `8.5` → `[8.5, 9)` (up to but excluding the next major).
/// - `8.5-` → `[8.5, ∞)`.
/// - `8.5-9.0` → `[8.5, 9.0)`.
fn vsatisfies(version: &str, req: &str) -> bool {
    let req = req.trim();
    let (lo, hi) = if let Some((lo, hi)) = req.split_once('-') {
        let hi = hi.trim();
        (
            lo.trim().to_string(),
            if hi.is_empty() {
                None
            } else {
                Some(hi.to_string())
            },
        )
    } else {
        // Bare `X.Y` → upper bound is the next major version.
        let major = parse_version(req).first().copied().unwrap_or(0);
        (req.to_string(), Some(format!("{}", major + 1)))
    };
    if cmp_version(version, &lo) == Ordering::Less {
        return false;
    }
    match hi {
        Some(hi) => cmp_version(version, &hi) == Ordering::Less,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::vsatisfies;

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
