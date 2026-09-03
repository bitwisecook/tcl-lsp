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

//! Platform commands: `pwd`, `cd`, `file`, `glob`, `exec`.
//!
//! Path manipulation subcommands (`join`/`dirname`/`tail`/…) are pure string
//! operations; the query/mutation subcommands (`exists`/`mtime`/`mkdir`/…),
//! `pwd`/`cd`/`glob`, and `exec` reach the host through the [`tcl_platform`]
//! capability seam ([`Vm::host`](crate::interp::Vm)) — the filesystem/env on a
//! [`NativeHost`](crate::host_native::NativeHost), subprocess via
//! `host.process()` (absent → the faithful "unsupported" error).

use std::path::{Path, PathBuf};

use tcl_platform::Filesystem;
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("pwd", cmd_pwd);
    vm.register("cd", cmd_cd);
    vm.register("file", cmd_file);
    vm.register("glob", cmd_glob);
    vm.register("exec", cmd_exec);
}

/// `exec arg ?arg ...?` — run a subprocess via the shared
/// [`tcl_cmd_core::platform::exec`] body over `host.process()`. On a host
/// without subprocess support (every WASM target, a sandbox) it yields the
/// faithful "unsupported" Tcl error rather than running — the capability model
/// in action. The host handle is cloned (`host_rc`) so it can be passed
/// alongside the `&mut Vm` the shared helper takes as its `ValueOps`.
fn cmd_exec(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let host = vm.host_rc();
    match tcl_cmd_core::platform::exec(vm, &*host, args) {
        Ok(v) => ok(v),
        Err(e) => err(e.into_message()),
    }
}

fn cmd_pwd(vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    match vm.host().env().cwd() {
        Ok(p) => ok(Value::string(p)),
        Err(e) => err(format!("error getting working directory name: {e}")),
    }
}

fn cmd_cd(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let env = vm.host().env();
    let dir = match args {
        [] => env.get("HOME").unwrap_or_else(|| "/".to_string()),
        [d] => d.to_str().to_string(),
        _ => return err("wrong # args: should be \"cd ?dirName?\""),
    };
    match env.chdir(&dir) {
        Ok(()) => ok(Value::empty()),
        Err(e) => err(format!(
            "couldn't change working directory to \"{dir}\": {e}"
        )),
    }
}

/// Wrap a `tcl_cmd_core::path` op's byte-slice result as a `Value`. Paths come
/// from `Value`s (UTF-8), and the `/`/`.`-split slice stays valid UTF-8.
fn path_str(bytes: &[u8]) -> Completion<Value> {
    ok(Value::string(std::str::from_utf8(bytes).unwrap_or("")))
}

/// Resolve a `file` subcommand word to its canonical Tcl 9 name with Tcl's
/// unambiguous-prefix rule (`Tcl_GetIndexFromObj`): exact match wins, else a
/// unique prefix — so `file ext` resolves to `extension` (cmdAH.test). The
/// table is the full Tcl 9 `file` option set so ambiguity matches C even for
/// subcommands the VM does not yet implement (those resolve then fall through
/// to the unknown-subcommand arm). `None` ⇒ no match or ambiguous.
fn canonical_file_sub(sub: &str) -> Option<&'static str> {
    const SUBS: &[&str] = &[
        "atime",
        "attributes",
        "channels",
        "copy",
        "delete",
        "dirname",
        "executable",
        "exists",
        "extension",
        "isdirectory",
        "isfile",
        "join",
        "link",
        "lstat",
        "mkdir",
        "mtime",
        "nativename",
        "normalize",
        "owned",
        "pathtype",
        "readable",
        "readlink",
        "rename",
        "rootname",
        "separator",
        "size",
        "split",
        "stat",
        "system",
        "tail",
        "tempdir",
        "tempfile",
        "type",
        "volumes",
        "writable",
    ];
    if sub.is_empty() {
        return None;
    }
    if let Some(&exact) = SUBS.iter().find(|&&s| s == sub) {
        return Some(exact);
    }
    let mut found = None;
    let mut count = 0u32;
    for &s in SUBS {
        if s.starts_with(sub) {
            found = Some(s);
            count += 1;
        }
    }
    if count == 1 { found } else { None }
}

/// The platform-independent path-text subcommands of `file` (no filesystem
/// access): `join`, `dirname`, `tail`, `extension`, `rootname`, `split`,
/// `normalize`, `nativename`, `pathtype`, `separator`. The `/`-based path text
/// ops are the shared `tcl_cmd_core::path` core (platform-independent, unlike
/// the VM's old `std::path::Path` versions). Returns `None` for any other
/// subcommand so the caller falls through to the filesystem-backed ops.
fn file_path_op(vm: &mut Vm, canon: &str, rest: &[Value]) -> Option<Completion<Value>> {
    let s = |v: &Value| v.to_str().to_string();
    Some(match canon {
        "join" => ok(Value::string(file_join(rest))),
        "dirname" => match rest {
            [p] => path_str(tcl_cmd_core::path::dirname(p.to_str().as_bytes())),
            _ => err("wrong # args: should be \"file dirname name\""),
        },
        "tail" => match rest {
            [p] => path_str(tcl_cmd_core::path::tail(p.to_str().as_bytes())),
            _ => err("wrong # args: should be \"file tail name\""),
        },
        "extension" => match rest {
            [p] => path_str(tcl_cmd_core::path::extension(p.to_str().as_bytes())),
            _ => err("wrong # args: should be \"file extension name\""),
        },
        "rootname" => match rest {
            [p] => path_str(tcl_cmd_core::path::rootname(p.to_str().as_bytes())),
            _ => err("wrong # args: should be \"file rootname name\""),
        },
        "split" => match rest {
            [p] => ok(Value::list(
                split_path(&s(p)).into_iter().map(Value::string).collect(),
            )),
            _ => err("wrong # args: should be \"file split name\""),
        },
        "normalize" => match rest {
            [p] => {
                let cwd = vm.host().env().cwd().unwrap_or_else(|_| "/".to_string());
                ok(Value::string(normalize(&s(p), &cwd)))
            }
            _ => err("wrong # args: should be \"file normalize name\""),
        },
        "nativename" => match rest {
            [p] => ok(Value::string(s(p))),
            _ => err("wrong # args: should be \"file nativename name\""),
        },
        "pathtype" => match rest {
            [p] => ok(Value::string(
                if Path::new(&s(p)).is_absolute() {
                    "absolute"
                } else {
                    "relative"
                }
                .to_string(),
            )),
            _ => err("wrong # args: should be \"file pathtype name\""),
        },
        "separator" => ok(Value::string("/")),
        _ => return None,
    })
}

fn cmd_file(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"file subcommand ?arg ...?\"");
    };
    let s = |v: &Value| v.to_str().to_string();
    let sub_str = sub.to_str();
    let canon: &str = match canonical_file_sub(&sub_str) {
        Some(c) => c,
        None => &sub_str,
    };
    // Pure path-text subcommands (no filesystem access) are handled first; the
    // rest fall through to the filesystem-backed ops below.
    if let Some(c) = file_path_op(vm, canon, rest) {
        return c;
    }
    match canon {
        // -- filesystem queries --
        // `readable`/`writable`/`executable` only check existence (good enough
        // for the test host where files are owned by the runner).
        "exists" | "readable" | "writable" | "executable" => {
            bool_query(vm.host().filesystem(), rest, |fs, p| fs.exists(p))
        }
        "isdirectory" | "isdir" => bool_query(vm.host().filesystem(), rest, |fs, p| {
            fs.metadata(p).is_ok_and(|m| m.is_dir)
        }),
        "isfile" => bool_query(vm.host().filesystem(), rest, |fs, p| {
            fs.metadata(p).is_ok_and(|m| m.is_file)
        }),
        "size" => match rest {
            [p] => match vm
                .host()
                .filesystem()
                .and_then(|fs| fs.metadata(&s(p)).ok())
            {
                Some(m) => ok(Value::int(i64::try_from(m.len).unwrap_or(i64::MAX))),
                None => err(format!("could not read \"{}\": no such file", s(p))),
            },
            _ => err("wrong # args: should be \"file size name\""),
        },
        "mtime" => match rest {
            [p] => file_mtime(vm.host().filesystem(), &s(p)),
            _ => err("wrong # args: should be \"file mtime name ?time?\""),
        },
        // -- filesystem mutation --
        "mkdir" => {
            let Some(fs) = vm.host().filesystem() else {
                return err("can't create directory: filesystem not available");
            };
            for p in rest {
                if let Err(e) = fs.create_dir_all(&s(p)) {
                    return err(format!("can't create directory \"{}\": {e}", s(p)));
                }
            }
            ok(Value::empty())
        }
        "delete" => {
            // `file delete ?-force? ?--? name ...`
            let mut paths = rest;
            while let Some(first) = paths.first() {
                match &*first.to_str() {
                    "-force" | "--" => paths = &paths[1..],
                    _ => break,
                }
            }
            if let Some(fs) = vm.host().filesystem() {
                for p in paths {
                    // Removes a file or a directory (recursively); a missing
                    // target is ignored, matching `file delete`.
                    let _ = fs.remove(&s(p), true);
                }
            }
            ok(Value::empty())
        }
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
    }
}

/// A boolean `file` query through the host filesystem. A host without a
/// filesystem (`None`) answers `false` — nothing exists where there is no fs.
fn bool_query(
    fs: Option<&dyn Filesystem>,
    rest: &[Value],
    pred: impl Fn(&dyn Filesystem, &str) -> bool,
) -> Completion<Value> {
    match rest {
        [p] => ok(Value::bool(fs.is_some_and(|fs| pred(fs, &p.to_str())))),
        _ => err("wrong # args: should be \"file <op> name\""),
    }
}

fn file_mtime(fs: Option<&dyn Filesystem>, path: &str) -> Completion<Value> {
    match fs.and_then(|fs| fs.metadata(path).ok()) {
        Some(m) => ok(Value::int(m.mtime_secs)),
        None => err(format!(
            "could not read \"{path}\": no such file or directory"
        )),
    }
}

/// `file join a b c` — join components, an absolute component resets the path.
fn file_join(parts: &[Value]) -> String {
    let mut buf = PathBuf::new();
    for p in parts {
        let s = p.to_str();
        if s.starts_with('/') {
            buf = PathBuf::from(&*s);
        } else if !s.is_empty() {
            buf.push(&*s);
        }
    }
    buf.to_string_lossy().into_owned()
}

fn split_path(p: &str) -> Vec<String> {
    let mut out = Vec::new();
    if p.starts_with('/') {
        out.push("/".to_string());
    }
    for c in p.split('/').filter(|c| !c.is_empty()) {
        out.push(c.to_string());
    }
    out
}

/// `file normalize` — make absolute (against the cwd) and resolve `.`/`..`
/// lexically (no symlink resolution).
fn normalize(p: &str, cwd: &str) -> String {
    let base = if Path::new(p).is_absolute() {
        PathBuf::from(p)
    } else {
        PathBuf::from(cwd).join(p)
    };
    let mut parts: Vec<String> = Vec::new();
    for c in base.components() {
        match c {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::CurDir => {}
            std::path::Component::RootDir => parts.clear(),
            other => parts.push(other.as_os_str().to_string_lossy().into_owned()),
        }
    }
    format!("/{}", parts.join("/"))
}

/// `glob ?-nocomplain? ?-directory dir? ?--? pattern ...` — minimal matching
/// against the directory's entries.
fn cmd_glob(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // We never error on no match (effectively always `-nocomplain`).
    let mut dir: Option<String> = None;
    let mut join = false;
    let mut i = 0;
    while i < args.len() {
        match &*args[i].to_str() {
            "-nocomplain" => i += 1,
            "-join" => {
                join = true;
                i += 1;
            }
            "-directory" | "-dir" => {
                dir = args.get(i + 1).map(|v| v.to_str().to_string());
                i += 2;
            }
            "--" => {
                i += 1;
                break;
            }
            s if s.starts_with('-') => i += 1, // ignore other options
            _ => break,
        }
    }
    let patterns = &args[i..];
    let base = dir.clone().unwrap_or_else(|| ".".to_string());
    let mut results: Vec<String> = Vec::new();
    let Some(filesystem) = vm.host().filesystem() else {
        return ok(Value::list(Vec::new()));
    };
    if join {
        glob_join(filesystem, &base, patterns, &mut results);
    } else if let Ok(entries) = filesystem.read_dir(&base) {
        for name in entries {
            let matches = patterns.is_empty()
                || patterns
                    .iter()
                    .any(|p| tcl_syntax::glob::string_match(&p.to_str(), &name));
            if matches {
                let full = if dir.is_some() {
                    file_join(&[Value::string(base.clone()), Value::string(name.clone())])
                } else {
                    name
                };
                results.push(full);
            }
        }
    }
    results.sort();
    ok(Value::list(
        results.into_iter().map(Value::string).collect(),
    ))
}

fn glob_join(
    filesystem: &dyn Filesystem,
    directory: &str,
    patterns: &[Value],
    results: &mut Vec<String>,
) {
    let Some((pattern, remaining)) = patterns.split_first() else {
        results.push(directory.to_owned());
        return;
    };
    let Ok(entries) = filesystem.read_dir(directory) else {
        return;
    };
    for entry in entries {
        if tcl_syntax::glob::string_match(&pattern.to_str(), &entry) {
            let path = file_join(&[Value::string(directory.to_owned()), Value::string(entry)]);
            if remaining.is_empty() {
                results.push(path);
            } else {
                glob_join(filesystem, &path, remaining, results);
            }
        }
    }
}
