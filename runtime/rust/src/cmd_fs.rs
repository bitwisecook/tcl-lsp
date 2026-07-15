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

//! Filesystem commands (M2 / L2) — `source`, `file`, `glob`, `pwd`, `cd`.
//!
//! All filesystem and working-directory access goes through the capability host
//! ([`Interp::host`](crate::interp::Interp)) — the [`tcl_platform`]
//! `Filesystem`/`Env` traits — rather than direct `std::fs`/`std::env`. A native
//! build gets the std-backed `NativeHost`; the WASM targets get a restricted
//! host (a no-VFS browser answers `false`/"unsupported"). C refs:
//! `tclIOUtil.c`/`tclFileName.c` (`source`/`glob`), `tclFCmd.c`/`tclFileName.c`
//! (`file`). Toward loading the real `init.tcl`/`tcltest.tcl` (the M2 gate).
//!
//! Path handling is `/`-separated (Tcl's portable convention); fine on Unix /
//! WASI.

use tcl_platform::{Filesystem, HostError};

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::list;
use crate::obj::TclObj;

/// Register `source`, `file`, `glob`, `pwd`, `cd`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"source", source_cmd);
    interp.register_builtin(b"file", file_cmd);
    interp.register_builtin(b"glob", glob_cmd);
    interp.register_builtin(b"pwd", pwd_cmd);
    interp.register_builtin(b"cd", cd_cmd);
}

fn as_str(b: &[u8]) -> &str {
    core::str::from_utf8(b).unwrap_or("")
}

// -- source ----------------------------------------------------------------

/// `source ?-encoding name? ?-nopkg? fileName` — read and evaluate a file.
/// We are UTF-8 internally so `-encoding` is accepted and ignored; `-nopkg`
/// (Tcl 9's "don't register for `package files`") is likewise a no-op here.
fn source_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"source ?-encoding encoding? ?-nopkg? fileName";
    let mut i = 1;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-encoding" if i + 1 < argv.len() => i += 2,
            b"-nopkg" => i += 1,
            _ => break,
        }
    }
    if i != argv.len() - 1 {
        return interp.wrong_args(USAGE);
    }
    let path = obj_bytes(argv[i]);
    let read = interp
        .host()
        .filesystem()
        .map_or(Err(HostError::NotFound), |fs| fs.read(as_str(&path)));
    match read {
        Ok(bytes) => interp.eval_sourced(&bytes, &path),
        Err(e) => {
            let mut m = b"couldn't read file \"".to_vec();
            m.extend_from_slice(&path);
            m.extend_from_slice(b"\": ");
            m.extend_from_slice(e.reason().as_bytes());
            interp.set_error(&m)
        }
    }
}

// -- file ------------------------------------------------------------------

/// The `file` ensemble's subcommands (for the unambiguous-prefix resolution Tcl
/// ensembles do: `file isdir` → `isdirectory`).
const FILE_SUBCOMMANDS: &[&[u8]] = &[
    b"dirname",
    b"tail",
    b"rootname",
    b"extension",
    b"join",
    b"split",
    b"normalize",
    b"separator",
    b"nativename",
    b"exists",
    b"isdirectory",
    b"isfile",
    b"readable",
    b"writable",
    b"executable",
    b"pathtype",
    b"delete",
    b"mkdir",
    b"size",
    b"type",
];

fn file_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"file subcommand ?arg ...?");
    }
    let raw = obj_bytes(argv[1]);
    // Ensemble prefix resolution: an exact name wins; otherwise a unique prefix.
    let sub: Vec<u8> = if FILE_SUBCOMMANDS.contains(&raw.as_slice()) {
        raw.clone()
    } else {
        let hits: Vec<&&[u8]> = FILE_SUBCOMMANDS
            .iter()
            .filter(|s| s.starts_with(raw.as_slice()))
            .collect();
        if hits.len() == 1 {
            hits[0].to_vec()
        } else {
            raw.clone() // 0 or ambiguous → fall through to the error arm
        }
    };
    let arg = |n: usize| argv.get(n).map(|&a| obj_bytes(a));
    match sub.as_slice() {
        // The pure `/`-based path text ops are the shared `tcl_cmd_core::path` core.
        b"dirname" => str_result(
            interp,
            tcl_cmd_core::path::dirname(&arg(2).unwrap_or_default()),
        ),
        b"tail" => str_result(
            interp,
            tcl_cmd_core::path::tail(&arg(2).unwrap_or_default()),
        ),
        b"rootname" => str_result(
            interp,
            tcl_cmd_core::path::rootname(&arg(2).unwrap_or_default()),
        ),
        b"extension" => str_result(
            interp,
            tcl_cmd_core::path::extension(&arg(2).unwrap_or_default()),
        ),
        b"join" => {
            let parts: Vec<Vec<u8>> = argv[2..].iter().map(|&a| obj_bytes(a)).collect();
            str_result(interp, &join(&parts))
        }
        b"split" => {
            let parts = split_path(&arg(2).unwrap_or_default());
            let objs: Vec<*mut TclObj> = parts.iter().map(|p| new_string(p)).collect();
            interp.set_result(list::new_list_obj(&objs));
            Code::Ok
        }
        b"normalize" => {
            let cwd = interp
                .host()
                .env()
                .cwd()
                .unwrap_or_else(|_| String::from("/"));
            let norm = normalize(&arg(2).unwrap_or_default(), cwd.as_bytes());
            str_result(interp, &norm)
        }
        b"separator" => str_result(interp, b"/"),
        b"nativename" => str_result(interp, &arg(2).unwrap_or_default()),
        b"exists" => {
            let e = fs_exists(interp, &arg(2).unwrap_or_default());
            bool_result(interp, e)
        }
        b"isdirectory" => {
            let d = fs_meta(interp, &arg(2).unwrap_or_default()).is_some_and(|m| m.is_dir);
            bool_result(interp, d)
        }
        b"isfile" => {
            let f = fs_meta(interp, &arg(2).unwrap_or_default()).is_some_and(|m| m.is_file);
            bool_result(interp, f)
        }
        b"readable" | b"writable" | b"executable" => {
            // Approximate: existence (fine-grained perms are deferred).
            let e = fs_exists(interp, &arg(2).unwrap_or_default());
            bool_result(interp, e)
        }
        // `file pathtype name` — pure-syntax classification.
        b"pathtype" => {
            let p = arg(2).unwrap_or_default();
            str_result(
                interp,
                if p.first() == Some(&b'/') {
                    b"absolute"
                } else {
                    b"relative"
                },
            )
        }
        b"delete" => file_delete(interp, argv),
        b"mkdir" => {
            for &a in &argv[2..] {
                let p = obj_bytes(a);
                if p.is_empty() {
                    continue;
                }
                let res = interp
                    .host()
                    .filesystem()
                    .map_or(Err(HostError::Unsupported), |fs| {
                        fs.create_dir_all(as_str(&p))
                    });
                if let Err(e) = res {
                    return host_fs_error(interp, b"can't create directory", &p, &e.reason());
                }
            }
            interp.set_result_bytes(b"");
            Code::Ok
        }
        b"size" => {
            let p = arg(2).unwrap_or_default();
            let meta = interp
                .host()
                .filesystem()
                .map_or(Err(HostError::NotFound), |fs| fs.metadata(as_str(&p)));
            match meta {
                Ok(m) => {
                    interp.set_result(crate::obj::new_wide_int_obj(m.len as i64));
                    Code::Ok
                }
                Err(e) => host_fs_error(interp, b"could not read", &p, &e.reason()),
            }
        }
        b"type" => {
            let p = arg(2).unwrap_or_default();
            let meta = interp
                .host()
                .filesystem()
                .map_or(Err(HostError::NotFound), |fs| {
                    fs.symlink_metadata(as_str(&p))
                });
            match meta {
                Ok(m) => str_result(
                    interp,
                    if m.is_symlink {
                        b"link"
                    } else if m.is_dir {
                        b"directory"
                    } else {
                        b"file"
                    },
                ),
                Err(e) => host_fs_error(interp, b"could not read", &p, &e.reason()),
            }
        }
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be delete, dirname, executable, exists, extension, isdirectory, isfile, join, mkdir, nativename, normalize, pathtype, readable, rootname, separator, size, split, tail, type, or writable");
            interp.set_error(&m)
        }
    }
}

/// `file delete ?-force? ?--? ?pathname ...?` — delete files / directories.
/// A non-existent path is silently ignored; `-force` removes non-empty
/// directories recursively. Returns the empty string.
fn file_delete(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut force = false;
    let mut i = 2;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-force" => force = true,
            b"--" => {
                i += 1;
                break;
            }
            _ => break,
        }
        i += 1;
    }
    for &a in &argv[i..] {
        let p = obj_bytes(a);
        // The host's `remove` already takes its own (non-following)
        // `symlink_metadata` to decide file-vs-recursive-dir, so a symlink to a
        // directory is unlinked rather than recursed — matching the prior
        // inline logic. A missing path yields `NotFound`, which we swallow
        // (`file delete` ignores absent targets).
        let res = interp
            .host()
            .filesystem()
            .map_or(Ok(()), |fs| fs.remove(as_str(&p), force));
        match res {
            Ok(()) | Err(HostError::NotFound) => {}
            Err(e) => return host_fs_error(interp, b"error deleting", &p, &e.reason()),
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// A `file` I/O error in Tcl's shape, with the reason already rendered from a
/// [`HostError`] (the [`Filesystem`](tcl_platform::Filesystem)-routed twin of
/// [`fs_error`], which renders from a `std::io::Error`).
fn host_fs_error(interp: &mut Interp, prefix: &[u8], path: &[u8], reason: &str) -> Code {
    let mut m = prefix.to_vec();
    m.extend_from_slice(b" \"");
    m.extend_from_slice(path);
    m.extend_from_slice(b"\": ");
    m.extend_from_slice(reason.as_bytes());
    interp.set_error(&m)
}

/// Existence via the host filesystem (`false` when the host provides none, e.g.
/// a no-VFS browser — nothing exists where there is no filesystem).
fn fs_exists(interp: &Interp, path: &[u8]) -> bool {
    interp
        .host()
        .filesystem()
        .is_some_and(|fs| fs.exists(as_str(path)))
}

/// Metadata via the host filesystem (`None` when the host provides none, or on
/// any error — callers needing the failure reason call `metadata` directly).
fn fs_meta(interp: &Interp, path: &[u8]) -> Option<tcl_platform::Metadata> {
    interp
        .host()
        .filesystem()
        .and_then(|fs| fs.metadata(as_str(path)).ok())
}

fn str_result(interp: &mut Interp, s: &[u8]) -> Code {
    interp.set_result_bytes(s);
    Code::Ok
}

fn bool_result(interp: &mut Interp, b: bool) -> Code {
    interp.set_result_bytes(if b { b"1" } else { b"0" });
    Code::Ok
}

/// Trim trailing `/` but keep a lone root `/`.
fn trim_trailing(p: &[u8]) -> &[u8] {
    let mut end = p.len();
    while end > 1 && p[end - 1] == b'/' {
        end -= 1;
    }
    &p[..end]
}

fn join(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        if part.starts_with(b"/") || out.is_empty() {
            out = part.clone(); // absolute component resets
        } else {
            if out.last() != Some(&b'/') {
                out.push(b'/');
            }
            out.extend_from_slice(part);
        }
    }
    out
}

fn split_path(p: &[u8]) -> Vec<Vec<u8>> {
    let mut parts: Vec<Vec<u8>> = Vec::new();
    if p.starts_with(b"/") {
        parts.push(b"/".to_vec());
    }
    for seg in p.split(|&c| c == b'/') {
        if !seg.is_empty() {
            parts.push(seg.to_vec());
        }
    }
    parts
}

/// Lexical normalize: make absolute (against `cwd`) and resolve `.`/`..` without
/// requiring the path to exist. `cwd` is the host's working directory (`pwd`).
fn normalize(p: &[u8], cwd: &[u8]) -> Vec<u8> {
    let mut abs: Vec<u8> = if p.starts_with(b"/") {
        p.to_vec()
    } else {
        let mut base = cwd.to_vec();
        base.push(b'/');
        base.extend_from_slice(p);
        base
    };
    abs = trim_trailing(&abs).to_vec();
    let mut stack: Vec<&[u8]> = Vec::new();
    for seg in abs.split(|&c| c == b'/') {
        match seg {
            b"" | b"." => {}
            b".." => {
                stack.pop();
            }
            s => stack.push(s),
        }
    }
    let mut out = Vec::new();
    for s in stack {
        out.push(b'/');
        out.extend_from_slice(s);
    }
    if out.is_empty() {
        out.push(b'/');
    }
    out
}

// -- pwd / cd --------------------------------------------------------------

fn pwd_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 1 {
        return interp.wrong_args(b"pwd");
    }
    let cwd = interp.host().env().cwd();
    match cwd {
        Ok(d) => str_result(interp, d.as_bytes()),
        Err(_) => interp.set_error(b"error getting working directory name"),
    }
}

fn cd_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 2 {
        return interp.wrong_args(b"cd ?dirName?");
    }
    let dir = argv
        .get(1)
        .map(|&a| obj_bytes(a))
        .unwrap_or_else(|| b"/".to_vec());
    let res = interp.host().env().chdir(as_str(&dir));
    match res {
        Ok(()) => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        Err(e) => {
            let mut m = b"couldn't change working directory to \"".to_vec();
            m.extend_from_slice(&dir);
            m.extend_from_slice(b"\": ");
            m.extend_from_slice(e.reason().as_bytes());
            interp.set_error(&m)
        }
    }
}

// -- glob ------------------------------------------------------------------

/// `glob ?-nocomplain? ?-directory dir? ?-tails? ?-join? ?--? pattern ...` —
/// filesystem name matching.
fn glob_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut nocomplain = false;
    let mut tails = false;
    let mut join_mode = false;
    let mut directory: Option<Vec<u8>> = None;
    let mut types: Vec<u8> = Vec::new();
    let mut i = 1;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-nocomplain" => nocomplain = true,
            b"-tails" => tails = true,
            b"-join" => join_mode = true,
            b"-directory" | b"-path" => {
                i += 1;
                directory = argv.get(i).map(|&a| obj_bytes(a));
            }
            b"-type" | b"-types" => {
                i += 1;
                // The value is a list of type specifiers (`d`, `f`, `r`, …); a
                // name must satisfy every requested test (`tclFileName.c`).
                if let Some(&a) = argv.get(i) {
                    types = crate::parse::split_list(&obj_bytes(a))
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|s| (s.len() == 1).then_some(s[0]))
                        .collect();
                }
            }
            b"--" => {
                i += 1;
                break;
            }
            opt if opt.starts_with(b"-") => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(opt);
                m.extend_from_slice(
                    b"\": must be -directory, -join, -nocomplain, -path, -tails, -types, or --",
                );
                return interp.set_error(&m);
            }
            _ => break,
        }
        i += 1;
    }
    // Patterns: each remaining arg, or (with -join) all joined into one.
    let pats: Vec<Vec<u8>> = argv[i..].iter().map(|&a| obj_bytes(a)).collect();
    let patterns: Vec<Vec<u8>> = if join_mode { vec![join(&pats)] } else { pats };

    let base = directory.clone();
    let mut hits: Vec<Vec<u8>> = Vec::new();
    // The host filesystem drives directory listing + the `-types` stats. An
    // independent `Rc` handle (`host`) keeps the borrow off `interp`, which the
    // result/error tail still needs mutably.
    let host = interp.host();
    let fs = host.filesystem();
    for pat in &patterns {
        glob_one(fs, base.as_deref(), pat, tails, &types, &mut hits);
    }
    hits.sort();
    hits.dedup();
    if hits.is_empty() && !nocomplain && patterns.iter().any(|p| !p.is_empty()) {
        let mut m = b"no files matched glob pattern".to_vec();
        if patterns.len() == 1 {
            m.extend_from_slice(b" \"");
            m.extend_from_slice(&patterns[0]);
            m.push(b'"');
        } else {
            m.extend_from_slice(b"s");
        }
        return interp.set_error(&m);
    }
    let objs: Vec<*mut TclObj> = hits.iter().map(|h| new_string(h)).collect();
    interp.set_result(list::new_list_obj(&objs));
    Code::Ok
}

/// Match one glob pattern against the filesystem, pushing results (full paths,
/// or just the tail when `tails`) onto `hits`. `fs` is the host filesystem
/// (`None` ⇒ no VFS ⇒ no matches).
fn glob_one(
    fs: Option<&dyn Filesystem>,
    directory: Option<&[u8]>,
    pattern: &[u8],
    tails: bool,
    types: &[u8],
    hits: &mut Vec<Vec<u8>>,
) {
    // Starting directory + whether results are absolute.
    let (start, abs_prefix): (Vec<u8>, Vec<u8>) = if pattern.starts_with(b"/") {
        (b"/".to_vec(), b"/".to_vec())
    } else if let Some(d) = directory {
        (d.to_vec(), if tails { Vec::new() } else { d.to_vec() })
    } else {
        (b".".to_vec(), Vec::new())
    };
    let segs: Vec<&[u8]> = pattern
        .split(|&c| c == b'/')
        .filter(|s| !s.is_empty())
        .collect();
    walk(fs, &start, &abs_prefix, &segs, 0, types, hits);
}

/// Recursively match path segments `segs[idx..]` under `dir`, accumulating the
/// display path (`prefix`).
fn walk(
    fs: Option<&dyn Filesystem>,
    dir: &[u8],
    prefix: &[u8],
    segs: &[&[u8]],
    idx: usize,
    types: &[u8],
    hits: &mut Vec<Vec<u8>>,
) {
    if idx >= segs.len() {
        if !prefix.is_empty() {
            hits.push(prefix.to_vec());
        }
        return;
    }
    let seg = segs[idx];
    let last = idx + 1 == segs.len();
    let Some(names) = fs.and_then(|fs| fs.read_dir(as_str(dir)).ok()) else {
        return;
    };
    for name in names {
        let name_b = name.as_bytes();
        if !glob_seg_match(seg, name_b) {
            continue;
        }
        let mut child_prefix = prefix.to_vec();
        if !child_prefix.is_empty() && child_prefix.last() != Some(&b'/') {
            child_prefix.push(b'/');
        }
        child_prefix.extend_from_slice(name_b);
        // Full path of the entry (for the `-types` stat and for recursion).
        let mut child_path = dir.to_vec();
        if child_path.last() != Some(&b'/') {
            child_path.push(b'/');
        }
        child_path.extend_from_slice(name_b);
        if last {
            // `-types`: keep only entries that satisfy every requested test.
            if !entry_matches_types(fs, &child_path, types) {
                continue;
            }
            hits.push(child_prefix);
        } else {
            walk(fs, &child_path, &child_prefix, segs, idx + 1, types, hits);
        }
    }
}

/// Whether the entry at `path` satisfies every `-types` specifier. An empty list
/// matches everything. Recognised portably: file-kind `d f l` and permission
/// `r w x` (mirrors `tclFileName.c`'s `GLOB_TYPE_*`). A name passes only if it
/// matches every requested test.
///
/// The Unix special-device kinds `p s b c` (fifo/socket/block/char) are **not**
/// expressible through the portable [`Filesystem`] seam — a WASI/browser host
/// has none — so they never match here (the maturity-asymmetry tax: the prior
/// native-only path could match them via `std::os::unix`). `r`/`w` stay
/// best-effort `true`; `x` reads the host's executable bit.
fn entry_matches_types(fs: Option<&dyn Filesystem>, path: &[u8], types: &[u8]) -> bool {
    if types.is_empty() {
        return true;
    }
    // `symlink_metadata` so `l` (and the kind tests) see the link itself, like C.
    let Some(meta) = fs.and_then(|fs| fs.symlink_metadata(as_str(path)).ok()) else {
        return false;
    };
    for &t in types {
        let ok = match t {
            b'd' => meta.is_dir,
            b'f' => meta.is_file,
            b'l' => meta.is_symlink,
            b'r' | b'w' => true, // permission probes — best-effort (assume yes)
            b'x' => meta.executable,
            b'p' | b's' | b'b' | b'c' => false, // special devices: not portable
            _ => true,                          // unknown specifier: don't exclude
        };
        if !ok {
            return false;
        }
    }
    true
}

/// Whether a single path segment matches a glob segment. A literal segment (no
/// glob metacharacters) matches by equality; otherwise by `string match`.
fn glob_seg_match(pat: &[u8], name: &[u8]) -> bool {
    if pat.iter().any(|&c| matches!(c, b'*' | b'?' | b'[')) {
        // Hidden files (leading `.`) only match an explicit leading `.` (Unix).
        if name.starts_with(b".") && !pat.starts_with(b".") {
            return false;
        }
        tcl_syntax::glob::string_match(as_str(pat), as_str(name))
    } else {
        pat == name
    }
}

#[cfg(test)]
mod tests {
    use crate::interp::{Code, Interp};

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn file_mkdir_size_type_delete() {
        let mut i = Interp::new();
        let d = format!("/tmp/rtfs_{}", std::process::id());
        ok(&mut i, format!("file delete -force {d}").as_bytes());
        ok(&mut i, format!("file mkdir {d}/sub").as_bytes());
        assert_eq!(
            ok(&mut i, format!("file isdirectory {d}/sub").as_bytes()),
            b"1"
        );
        assert_eq!(
            ok(&mut i, format!("file type {d}").as_bytes()),
            b"directory"
        );
        assert_eq!(ok(&mut i, b"file pathtype /x"), b"absolute");
        assert_eq!(ok(&mut i, b"file pathtype rel"), b"relative");
        // Deleting a missing path is a silent no-op; size of a missing path errors.
        ok(&mut i, format!("file delete {d}/nope").as_bytes());
        assert_eq!(
            i.eval_str(format!("file size {d}/nope").as_bytes()),
            Code::Error
        );
        ok(&mut i, format!("file delete -force {d}").as_bytes());
        assert_eq!(ok(&mut i, format!("file exists {d}").as_bytes()), b"0");
    }

    #[test]
    fn file_path_ops() {
        let mut i = Interp::new();
        assert_eq!(ok(&mut i, b"file dirname /a/b/c"), b"/a/b");
        assert_eq!(ok(&mut i, b"file dirname a"), b".");
        assert_eq!(ok(&mut i, b"file dirname /a"), b"/");
        assert_eq!(ok(&mut i, b"file tail /a/b/c"), b"c");
        assert_eq!(ok(&mut i, b"file tail c"), b"c");
        assert_eq!(ok(&mut i, b"file join /a b c"), b"/a/b/c");
        assert_eq!(ok(&mut i, b"file join a /b"), b"/b");
        assert_eq!(ok(&mut i, b"file extension foo.tcl"), b".tcl");
        assert_eq!(ok(&mut i, b"file rootname /x/foo.tcl"), b"/x/foo");
        assert_eq!(ok(&mut i, b"file split /a/b/c"), b"/ a b c");
        assert_eq!(ok(&mut i, b"file normalize /a/./b/../c"), b"/a/c");
    }

    #[test]
    fn source_evaluates_a_file() {
        let mut i = Interp::new();
        let dir = std::env::temp_dir();
        let path = dir.join(format!("tclrt_src_{}.tcl", std::process::id()));
        std::fs::write(&path, b"set sourced 42\nreturn done\n").unwrap();
        let cmd = format!("source {}", path.display());
        assert_eq!(ok(&mut i, cmd.as_bytes()), b"done"); // top-level return → ok
        assert_eq!(ok(&mut i, b"set sourced"), b"42");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn glob_finds_files() {
        let mut i = Interp::new();
        let dir = std::env::temp_dir().join(format!("tclrt_glob_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.tcl"), b"").unwrap();
        std::fs::write(dir.join("b.txt"), b"").unwrap();
        let cmd = format!("glob -nocomplain -tails -directory {} *.tcl", dir.display());
        assert_eq!(ok(&mut i, cmd.as_bytes()), b"a.tcl");
        let none = format!("glob -nocomplain -directory {} *.zzz", dir.display());
        assert_eq!(ok(&mut i, none.as_bytes()), b"");
        std::fs::remove_dir_all(&dir).ok();
    }
}
