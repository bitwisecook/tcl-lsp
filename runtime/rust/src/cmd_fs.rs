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

/// `source ?-encoding name? fileName` — read and evaluate a file.
fn source_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // Skip a leading `-encoding enc` (we are UTF-8 internally).
    let path_obj = match argv.len() {
        2 => argv[1],
        4 if obj_bytes(argv[1]).as_slice() == b"-encoding" => argv[3],
        _ => return wrong_args(interp, b"source ?-encoding encoding? fileName"),
    };
    let path = obj_bytes(path_obj);
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

fn file_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"file subcommand ?arg ...?");
    }
    let sub = obj_bytes(argv[1]);
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
        b"readable" | b"writable" => {
            // Approximate: existence (fine-grained perms are deferred).
            bool_result(
                interp,
                Path::new(as_str(&arg(2).unwrap_or_default())).exists(),
            )
        }
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be dirname, exists, extension, isdirectory, isfile, join, nativename, normalize, readable, rootname, separator, split, tail, or writable");
            interp.set_error(&m)
        }
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
            b"--" => {
                i += 1;
                break;
            }
            opt if opt.starts_with(b"-") => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(opt);
                m.extend_from_slice(b"\": must be -directory, -join, -nocomplain, -tails, or --");
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
        glob_one(base.as_deref(), pat, tails, &mut hits);
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
fn glob_one(directory: Option<&[u8]>, pattern: &[u8], tails: bool, hits: &mut Vec<Vec<u8>>) {
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
    walk(&start, &abs_prefix, &segs, 0, hits);
}

/// Recursively match path segments `segs[idx..]` under `dir`, accumulating the
/// display path (`prefix`).
fn walk(dir: &[u8], prefix: &[u8], segs: &[&[u8]], idx: usize, hits: &mut Vec<Vec<u8>>) {
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
            hits.push(child_prefix);
        } else {
            let mut child_dir = dir.to_vec();
            child_dir.push(b'/');
            child_dir.extend_from_slice(name_b);
            walk(&child_dir, &child_prefix, segs, idx + 1, hits);
        }
    }
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
