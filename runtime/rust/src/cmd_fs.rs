//! Filesystem commands (M2 / L2) — `source`, `file`, `glob`, `pwd`, `cd`.
//!
//! The "VFS" is the host filesystem via `std::fs`/`std::env` for the native /
//! `wasm32-wasip1` build (a non-WASI shim can swap in later). C refs:
//! `tclIOUtil.c`/`tclFileName.c` (`source`/`glob`), `tclFCmd.c`/`tclFileName.c`
//! (`file`). Toward loading the real `init.tcl`/`tcltest.tcl` (the M2 gate).
//!
//! Path handling is `/`-separated (Tcl's portable convention); fine on Unix /
//! WASI. See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::path::Path;

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

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
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
        return wrong_args(interp, USAGE);
    }
    let path = obj_bytes(argv[i]);
    match std::fs::read(as_str(&path)) {
        Ok(bytes) => interp.eval_sourced(&bytes, &path),
        Err(e) => {
            let mut m = b"couldn't read file \"".to_vec();
            m.extend_from_slice(&path);
            m.extend_from_slice(b"\": ");
            m.extend_from_slice(io_reason(&e).as_bytes());
            interp.set_error(&m)
        }
    }
}

fn io_reason(e: &std::io::Error) -> &'static str {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => "no such file or directory",
        ErrorKind::PermissionDenied => "permission denied",
        _ => "I/O error",
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
        return wrong_args(interp, b"file subcommand ?arg ...?");
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
        b"dirname" => str_result(interp, &dirname(&arg(2).unwrap_or_default())),
        b"tail" => str_result(interp, &tail(&arg(2).unwrap_or_default())),
        b"rootname" => str_result(interp, &rootname(&arg(2).unwrap_or_default())),
        b"extension" => str_result(interp, &extension(&arg(2).unwrap_or_default())),
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
        b"normalize" => str_result(interp, &normalize(&arg(2).unwrap_or_default())),
        b"separator" => str_result(interp, b"/"),
        b"nativename" => str_result(interp, &arg(2).unwrap_or_default()),
        b"exists" => bool_result(
            interp,
            Path::new(as_str(&arg(2).unwrap_or_default())).exists(),
        ),
        b"isdirectory" => bool_result(
            interp,
            Path::new(as_str(&arg(2).unwrap_or_default())).is_dir(),
        ),
        b"isfile" => bool_result(
            interp,
            Path::new(as_str(&arg(2).unwrap_or_default())).is_file(),
        ),
        b"readable" | b"writable" | b"executable" => {
            // Approximate: existence (fine-grained perms are deferred).
            bool_result(
                interp,
                Path::new(as_str(&arg(2).unwrap_or_default())).exists(),
            )
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
                if let Err(e) = std::fs::create_dir_all(as_str(&p)) {
                    return fs_error(interp, b"can't create directory", &p, &e);
                }
            }
            interp.set_result_bytes(b"");
            Code::Ok
        }
        b"size" => {
            let p = arg(2).unwrap_or_default();
            match std::fs::metadata(as_str(&p)) {
                Ok(m) => {
                    interp.set_result(crate::obj::new_wide_int_obj(m.len() as i64));
                    Code::Ok
                }
                Err(e) => fs_error(interp, b"could not read", &p, &e),
            }
        }
        b"type" => {
            let p = arg(2).unwrap_or_default();
            match std::fs::symlink_metadata(as_str(&p)) {
                Ok(m) => {
                    let t = m.file_type();
                    str_result(
                        interp,
                        if t.is_symlink() {
                            b"link"
                        } else if t.is_dir() {
                            b"directory"
                        } else {
                            b"file"
                        },
                    )
                }
                Err(e) => fs_error(interp, b"could not read", &p, &e),
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
        let path = Path::new(as_str(&p));
        let meta = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(_) => continue, // not there ⇒ nothing to delete (no error)
        };
        let res = if meta.is_dir() && !meta.file_type().is_symlink() {
            if force {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_dir(path)
            }
        } else {
            std::fs::remove_file(path)
        };
        if let Err(e) = res {
            return fs_error(interp, b"error deleting", &p, &e);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// A `file` I/O error in Tcl's shape: `<prefix> "<path>": <reason>` (the prefix
/// is operation-specific, e.g. `could not read` / `can't create directory`).
fn fs_error(interp: &mut Interp, prefix: &[u8], path: &[u8], e: &std::io::Error) -> Code {
    let mut m = prefix.to_vec();
    m.extend_from_slice(b" \"");
    m.extend_from_slice(path);
    m.extend_from_slice(b"\": ");
    m.extend_from_slice(io_error_reason(e).as_bytes());
    interp.set_error(&m)
}

/// The POSIX-style message Tcl uses for an `errno` (the common cases).
fn io_error_reason(e: &std::io::Error) -> &'static str {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => "no such file or directory",
        PermissionDenied => "permission denied",
        AlreadyExists => "file already exists",
        _ => "i/o error",
    }
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

fn dirname(p: &[u8]) -> Vec<u8> {
    let p = trim_trailing(p);
    match p.iter().rposition(|&c| c == b'/') {
        None => b".".to_vec(),
        Some(0) => b"/".to_vec(),
        Some(i) => p[..i].to_vec(),
    }
}

fn tail(p: &[u8]) -> Vec<u8> {
    let p = trim_trailing(p);
    match p.iter().rposition(|&c| c == b'/') {
        None => p.to_vec(),
        Some(i) => p[i + 1..].to_vec(),
    }
}

fn extension(p: &[u8]) -> Vec<u8> {
    let t = tail(p);
    match t.iter().rposition(|&c| c == b'.') {
        Some(i) if i > 0 => t[i..].to_vec(),
        _ => Vec::new(),
    }
}

fn rootname(p: &[u8]) -> Vec<u8> {
    let ext = extension(p);
    p[..p.len() - ext.len()].to_vec()
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

/// Lexical normalize: make absolute (against `pwd`) and resolve `.`/`..` without
/// requiring the path to exist.
fn normalize(p: &[u8]) -> Vec<u8> {
    let mut abs: Vec<u8> = if p.starts_with(b"/") {
        p.to_vec()
    } else {
        let mut base = std::env::current_dir()
            .ok()
            .and_then(|d| d.to_str().map(|s| s.as_bytes().to_vec()))
            .unwrap_or_else(|| b"/".to_vec());
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
        return wrong_args(interp, b"pwd");
    }
    match std::env::current_dir() {
        Ok(d) => str_result(interp, d.to_string_lossy().as_bytes()),
        Err(_) => interp.set_error(b"error getting working directory name"),
    }
}

fn cd_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 2 {
        return wrong_args(interp, b"cd ?dirName?");
    }
    let dir = argv
        .get(1)
        .map(|&a| obj_bytes(a))
        .unwrap_or_else(|| b"/".to_vec());
    match std::env::set_current_dir(as_str(&dir)) {
        Ok(()) => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        Err(e) => {
            let mut m = b"couldn't change working directory to \"".to_vec();
            m.extend_from_slice(&dir);
            m.extend_from_slice(b"\": ");
            m.extend_from_slice(io_reason(&e).as_bytes());
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
    for pat in &patterns {
        glob_one(base.as_deref(), pat, tails, &types, &mut hits);
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
/// or just the tail when `tails`) onto `hits`.
fn glob_one(
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
    walk(&start, &abs_prefix, &segs, 0, types, hits);
}

/// Recursively match path segments `segs[idx..]` under `dir`, accumulating the
/// display path (`prefix`).
fn walk(
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
    let Ok(entries) = std::fs::read_dir(as_str(dir)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let name_b = name.as_bytes();
        if !glob_seg_match(seg, name_b) {
            continue;
        }
        let mut child_prefix = prefix.to_vec();
        if !child_prefix.is_empty() && child_prefix.last() != Some(&b'/') {
            child_prefix.push(b'/');
        }
        child_prefix.extend_from_slice(name_b);
        if last {
            // `-types`: keep only entries that satisfy every requested test.
            if !entry_matches_types(&entry, types) {
                continue;
            }
            hits.push(child_prefix);
        } else {
            let mut child_dir = dir.to_vec();
            child_dir.push(b'/');
            child_dir.extend_from_slice(name_b);
            walk(&child_dir, &child_prefix, segs, idx + 1, types, hits);
        }
    }
}

/// Whether a directory entry satisfies every `-types` specifier. An empty list
/// matches everything. Recognised: file-kind `d f l p s b c` and permission
/// `r w x` (mirrors `tclFileName.c`'s `GLOB_TYPE_*`; the rarely-used
/// `{macintosh …}` forms are not modelled). A name passes only if it matches
/// every requested test — both any file-kind tests and the permission tests.
fn entry_matches_types(entry: &std::fs::DirEntry, types: &[u8]) -> bool {
    if types.is_empty() {
        return true;
    }
    // `symlink_metadata` so `l` (and the kind tests) see the link itself, like C.
    let Ok(meta) = entry.path().symlink_metadata() else {
        return false;
    };
    let ft = meta.file_type();
    for &t in types {
        let ok = match t {
            b'd' => ft.is_dir(),
            b'f' => ft.is_file(),
            b'l' => ft.is_symlink(),
            b'r' | b'w' => true, // permission probes — best-effort (assume yes)
            b'x' => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    meta.permissions().mode() & 0o111 != 0
                }
                #[cfg(not(unix))]
                {
                    true
                }
            }
            #[cfg(unix)]
            b'p' | b's' | b'b' | b'c' => {
                use std::os::unix::fs::FileTypeExt;
                match t {
                    b'p' => ft.is_fifo(),
                    b's' => ft.is_socket(),
                    b'b' => ft.is_block_device(),
                    _ => ft.is_char_device(),
                }
            }
            _ => true, // unknown specifier: don't exclude
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
