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

use tcl_dialect::model::{SurfaceQuery, surface_admits};
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

/// Runtime arity declarations (arguments after the `file` subcommand). The
/// command-backing gate parses this table and compares it with the registry;
/// keeping it next to the dispatch table prevents a present-but-partial
/// ensemble from reporting green.
const FILE_RUNTIME_ARITIES: &[(&str, u16, u16)] = &[
    ("atime", 1, 2),
    ("attributes", 1, 65535),
    ("channels", 0, 1),
    ("copy", 2, 65535),
    ("delete", 0, 65535),
    ("dirname", 1, 1),
    ("executable", 1, 1),
    ("exists", 1, 1),
    ("extension", 1, 1),
    ("home", 0, 1),
    ("isdirectory", 1, 1),
    ("isfile", 1, 1),
    ("join", 1, 65535),
    ("link", 1, 2),
    ("lstat", 1, 2),
    ("mkdir", 0, 65535),
    ("mtime", 1, 2),
    ("nativename", 1, 1),
    ("normalize", 1, 1),
    ("owned", 1, 1),
    ("pathtype", 1, 1),
    ("readable", 1, 1),
    ("readlink", 1, 1),
    ("rename", 2, 65535),
    ("rootname", 1, 1),
    ("separator", 0, 1),
    ("size", 1, 1),
    ("split", 1, 1),
    ("stat", 1, 2),
    ("system", 1, 1),
    ("tail", 1, 1),
    ("tempdir", 0, 1),
    ("tempfile", 0, 2),
    ("tildeexpand", 1, 1),
    ("type", 1, 1),
    ("volumes", 0, 0),
    ("writable", 1, 1),
];

fn file_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"file subcommand ?arg ...?");
    }
    let raw = obj_bytes(argv[1]);
    // The registry owns both the ensemble names and their release gates.
    // Resolve against the one environment selected for this interpreter so,
    // for example, Tcl 8.6 neither exposes Tcl 9's `tempdir` nor lets it
    // affect abbreviation uniqueness. The release name is a dialect *name*,
    // so it goes through the one ingress seam (`crate::environment`); the
    // ensemble is read off that environment's registry generation and gated
    // on its document authoring mask (ledger row B1).
    let profile =
        crate::environment::profile_for_dialect(interp.runtime_version().dialect_profile_name());
    let dialect = Some(crate::environment::surface_point(profile));
    let file_spec = crate::environment::store_for_profile(profile)
        .get("file")
        .expect("the Tcl registry contains file");
    let sub = match file_spec.resolve_subcommand_word(as_str(&raw), dialect, None, None) {
        tcl_registry::abbrev::KeywordMatch::Unique(name) => name.as_bytes().to_vec(),
        tcl_registry::abbrev::KeywordMatch::Ambiguous(_)
        | tcl_registry::abbrev::KeywordMatch::Unknown => {
            return file_unknown_subcommand(interp, &raw, file_spec, dialect);
        }
    };
    if let Some((_, min, max)) = FILE_RUNTIME_ARITIES
        .iter()
        .find(|(name, _, _)| *name == as_str(&sub))
    {
        // The registry arities count operands, while these four subcommands
        // accept one leading option.  Consume that option for the generic
        // fidelity guard before comparing the declared operand bounds.
        let option_count = matches!(sub.as_slice(), b"copy" | b"link" | b"rename")
            && argv.get(2).is_some_and(|a| {
                matches!(
                    obj_bytes(*a).as_slice(),
                    b"-force" | b"-hard" | b"-symbolic" | b"--"
                )
            });
        let argc = (argv.len().saturating_sub(2) - usize::from(option_count)) as u16;
        if argc < *min || argc > *max {
            let mut usage = b"file ".to_vec();
            usage.extend_from_slice(&sub);
            usage.extend_from_slice(b" ?arg ...?");
            return interp.wrong_args(&usage);
        }
    }
    let arg = |n: usize| argv.get(n).map(|&a| obj_bytes(a));
    match sub.as_slice() {
        b"atime" => file_time(interp, argv, true),
        b"attributes" => file_attributes(interp, argv),
        b"channels" => file_channels(interp, argv),
        b"copy" => file_copy(interp, argv),
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
        b"home" => file_home(interp, argv),
        b"join" => {
            let parts: Vec<Vec<u8>> = argv[2..].iter().map(|&a| obj_bytes(a)).collect();
            str_result(interp, &join(&parts))
        }
        b"link" => file_link(interp, argv),
        b"lstat" => file_stat(interp, argv, true),
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
        b"owned" => file_owned(interp, argv),
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
        b"readable" => {
            let e =
                fs_meta(interp, &arg(2).unwrap_or_default()).is_some_and(|m| m.mode & 0o444 != 0);
            bool_result(interp, e)
        }
        b"writable" => {
            let e =
                fs_meta(interp, &arg(2).unwrap_or_default()).is_some_and(|m| m.mode & 0o222 != 0);
            bool_result(interp, e)
        }
        b"executable" => {
            let e = fs_meta(interp, &arg(2).unwrap_or_default()).is_some_and(|m| m.executable);
            bool_result(interp, e)
        }
        b"readlink" => file_readlink(interp, argv),
        b"rename" => file_rename(interp, argv),
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
        b"mtime" => file_time(interp, argv, false),
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
        b"stat" => file_stat(interp, argv, false),
        b"system" => file_system(interp, argv),
        b"tempdir" => file_tempdir(interp, argv),
        b"tempfile" => file_tempfile(interp, argv),
        b"tildeexpand" => file_tildeexpand(interp, argv),
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
        b"volumes" => file_volumes(interp, argv),
        _ => unreachable!("the registry and runtime file surface are drift-gated"),
    }
}

fn file_unknown_subcommand(
    interp: &mut Interp,
    raw: &[u8],
    spec: &tcl_registry::CommandSpec,
    dialect: Option<SurfaceQuery<'_>>,
) -> Code {
    let visible: Vec<&str> = spec
        .subcommands
        .iter()
        .filter(|sub| {
            sub.surface
                .or(spec.surface)
                .is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
        })
        .map(|sub| sub.name)
        .collect();
    let mut message = b"unknown or ambiguous subcommand \"".to_vec();
    message.extend_from_slice(raw);
    message.extend_from_slice(b"\": must be ");
    for (index, name) in visible.iter().enumerate() {
        if index > 0 {
            message.extend_from_slice(if index + 1 == visible.len() {
                b", or "
            } else {
                b", "
            });
        }
        message.extend_from_slice(name.as_bytes());
    }
    interp.set_error(&message)
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

fn one_path(interp: &mut Interp, argv: &[*mut TclObj], usage: &[u8]) -> Option<Vec<u8>> {
    if argv.len() != 3 {
        interp.wrong_args(usage);
        None
    } else {
        Some(obj_bytes(argv[2]))
    }
}

fn file_time(interp: &mut Interp, argv: &[*mut TclObj], access: bool) -> Code {
    if argv.len() != 3 && argv.len() != 4 {
        return interp.wrong_args(if access {
            b"file atime name ?time?"
        } else {
            b"file mtime name ?time?"
        });
    }
    let path = obj_bytes(argv[2]);
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    if let Some(value) = argv.get(3) {
        let Ok(secs) = core::str::from_utf8(&obj_bytes(*value))
            .unwrap_or("")
            .parse::<i64>()
        else {
            return interp.set_error(b"expected integer but got invalid time");
        };
        return match fs.set_times(
            &as_str(&path),
            access.then_some(secs),
            (!access).then_some(secs),
        ) {
            Ok(()) => {
                interp.set_result_bytes(b"");
                Code::Ok
            }
            Err(e) => host_fs_error(interp, b"could not set file time for", &path, &e.reason()),
        };
    }
    match fs.metadata(&as_str(&path)) {
        Ok(m) => {
            let value = if access { m.atime_secs } else { m.mtime_secs };
            interp.set_result_bytes(value.to_string().as_bytes());
            Code::Ok
        }
        Err(e) => host_fs_error(interp, b"could not read", &path, &e.reason()),
    }
}

fn file_attributes(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"file attributes name ?option? ?value ...?");
    }
    let path = obj_bytes(argv[2]);
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    let meta = match fs.symlink_metadata(&as_str(&path)) {
        Ok(m) => m,
        Err(e) => return host_fs_error(interp, b"could not read", &path, &e.reason()),
    };
    if argv.len() == 3 {
        let tail = tcl_cmd_core::path::tail(&path);
        let entries = vec![
            (b"-group".as_slice(), meta.gid.to_string().into_bytes()),
            (b"-owner".as_slice(), meta.uid.to_string().into_bytes()),
            (
                b"-permissions".as_slice(),
                file_permissions(interp, meta.mode),
            ),
            (
                b"-readonly".as_slice(),
                if meta.mode & 0o222 == 0 {
                    b"1".to_vec()
                } else {
                    b"0".to_vec()
                },
            ),
            (
                b"-hidden".as_slice(),
                if tail.starts_with(b".") {
                    b"1".to_vec()
                } else {
                    b"0".to_vec()
                },
            ),
        ];
        let objs: Vec<*mut TclObj> = entries
            .iter()
            .flat_map(|(k, v)| [new_string(k), new_string(v)])
            .collect();
        interp.set_result(list::new_list_obj(&objs));
        return Code::Ok;
    }
    if argv.len() == 4 && obj_bytes(argv[3]) == b"-permissions" {
        interp.set_result_bytes(&file_permissions(interp, meta.mode));
        return Code::Ok;
    }
    interp.set_error(b"bad attribute option")
}

/// Tcl renders Unix `-permissions` as the complete mode word, using the
/// release's own octal spelling (`0NNN` through 8.6, `0oNNN` from 9.0).
fn file_permissions(interp: &Interp, mode: u32) -> Vec<u8> {
    let syntax = interp.runtime_version().number_syntax();
    let prefix = if syntax.has_decimal_prefix() {
        "0o"
    } else {
        "0"
    };
    format!("{prefix}{mode:o}").into_bytes()
}

fn file_channels(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"file channels ?pattern?");
    }
    let pattern = argv.get(2).map(|a| obj_bytes(*a));
    let names: Vec<Vec<u8>> = interp
        .channels
        .borrow()
        .names()
        .into_iter()
        .filter(|n| {
            pattern
                .as_ref()
                .is_none_or(|p| tcl_syntax::glob::string_match(as_str(p), as_str(n)))
        })
        .collect();
    let objs: Vec<*mut TclObj> = names.iter().map(|n| new_string(n)).collect();
    interp.set_result(list::new_list_obj(&objs));
    Code::Ok
}

fn file_copy(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
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
    if argv.len() - i < 2 {
        return interp.wrong_args(b"file copy ?-force? ?--? source ?source ...? target");
    }
    let target = obj_bytes(argv[argv.len() - 1]);
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    let target_is_dir = fs
        .symlink_metadata(&as_str(&target))
        .is_ok_and(|m| m.is_dir);
    let sources = &argv[i..argv.len() - 1];
    if sources.len() > 1 && !target_is_dir {
        return interp.set_error(b"target must be a directory");
    }
    for &source_obj in sources {
        let source = obj_bytes(source_obj);
        let destination = if target_is_dir {
            join(&[target.clone(), tcl_cmd_core::path::tail(&source).to_vec()])
        } else {
            target.clone()
        };
        let recursive = fs
            .symlink_metadata(&as_str(&source))
            .is_ok_and(|m| m.is_dir);
        if let Err(e) = fs.copy(&as_str(&source), &as_str(&destination), recursive, force) {
            return host_fs_error(interp, b"error copying", &source, &e.reason());
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn file_home(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"file home ?username?");
    }
    if argv.len() == 3 && !obj_bytes(argv[2]).is_empty() {
        return interp.set_error(b"could not determine home directory");
    }
    match interp.host().env().get("HOME") {
        Some(home) => str_result(interp, home.as_bytes()),
        None => interp.set_error(b"could not determine home directory"),
    }
}

fn file_link(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut hard = false;
    let mut i = 2;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-hard" => hard = true,
            b"-symbolic" => hard = false,
            _ => break,
        }
        i += 1;
    }
    let n = argv.len() - i;
    if n == 1 {
        return file_readlink_at(interp, &obj_bytes(argv[i]));
    }
    if n != 2 {
        return interp.wrong_args(b"file link ?-linktype? linkName ?target?");
    }
    let link = obj_bytes(argv[i]);
    let target = obj_bytes(argv[i + 1]);
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    match fs.link(&as_str(&link), &as_str(&target), hard) {
        Ok(()) => str_result(interp, &target),
        Err(e) => host_fs_error(interp, b"couldn't create link", &link, &e.reason()),
    }
}

fn file_owned(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(path) = one_path(interp, argv, b"file owned name") else {
        return Code::Error;
    };
    bool_result(interp, fs_exists(interp, &path))
}

fn file_readlink_at(interp: &mut Interp, path: &[u8]) -> Code {
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    match fs.readlink(&as_str(path)) {
        Ok(target) => str_result(interp, target.as_bytes()),
        Err(e) => host_fs_error(interp, b"couldn't read link", path, &e.reason()),
    }
}

fn file_readlink(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(path) = one_path(interp, argv, b"file readlink name") else {
        return Code::Error;
    };
    file_readlink_at(interp, &path)
}

fn file_rename(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
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
    if argv.len() - i < 2 {
        return interp.wrong_args(b"file rename ?-force? ?--? source ?source ...? target");
    }
    let target = obj_bytes(argv[argv.len() - 1]);
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    let target_is_dir = fs
        .symlink_metadata(&as_str(&target))
        .is_ok_and(|m| m.is_dir);
    let sources = &argv[i..argv.len() - 1];
    if sources.len() > 1 && !target_is_dir {
        return interp.set_error(b"target must be a directory");
    }
    for &source_obj in sources {
        let source = obj_bytes(source_obj);
        let destination = if target_is_dir {
            join(&[target.clone(), tcl_cmd_core::path::tail(&source).to_vec()])
        } else {
            target.clone()
        };
        if let Err(e) = fs.rename(&as_str(&source), &as_str(&destination), force) {
            return host_fs_error(interp, b"error renaming", &source, &e.reason());
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn file_stat(interp: &mut Interp, argv: &[*mut TclObj], no_follow: bool) -> Code {
    if argv.len() != 3 && argv.len() != 4 {
        return interp.wrong_args(if no_follow {
            b"file lstat name ?varName?"
        } else {
            b"file stat name ?varName?"
        });
    }
    let path = obj_bytes(argv[2]);
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    let meta = match if no_follow {
        fs.symlink_metadata(&as_str(&path))
    } else {
        fs.metadata(&as_str(&path))
    } {
        Ok(m) => m,
        Err(e) => return host_fs_error(interp, b"could not read", &path, &e.reason()),
    };
    let fields = stat_fields(meta);
    if let Some(var) = argv.get(3) {
        for (key, value) in &fields {
            let obj = new_string(value);
            if interp.var_set_elem(&obj_bytes(*var), key, obj).is_err() {
                crate::interp::drop_fresh(obj);
                return interp.set_error(b"can't set stat variable");
            }
        }
        interp.set_result_bytes(b"");
    } else {
        let objs: Vec<*mut TclObj> = fields
            .iter()
            .flat_map(|(k, v)| [new_string(k), new_string(v)])
            .collect();
        interp.set_result(list::new_list_obj(&objs));
    }
    Code::Ok
}

fn stat_fields(meta: tcl_platform::Metadata) -> Vec<(&'static [u8], Vec<u8>)> {
    vec![
        (b"dev", meta.dev.to_string().into_bytes()),
        (b"ino", meta.ino.to_string().into_bytes()),
        (b"mode", meta.mode.to_string().into_bytes()),
        (b"nlink", meta.nlink.to_string().into_bytes()),
        (b"uid", meta.uid.to_string().into_bytes()),
        (b"gid", meta.gid.to_string().into_bytes()),
        (b"size", meta.len.to_string().into_bytes()),
        (b"atime", meta.atime_secs.to_string().into_bytes()),
        (b"mtime", meta.mtime_secs.to_string().into_bytes()),
        (b"ctime", meta.ctime_secs.to_string().into_bytes()),
        (b"blocks", meta.blocks.to_string().into_bytes()),
        (b"blksize", meta.blksize.to_string().into_bytes()),
        (
            b"type",
            if meta.is_dir {
                b"directory".to_vec()
            } else if meta.is_symlink {
                b"link".to_vec()
            } else {
                b"file".to_vec()
            },
        ),
    ]
}

fn file_system(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(_path) = one_path(interp, argv, b"file system name") else {
        return Code::Error;
    };
    if interp.host().filesystem().is_none() {
        return interp.set_error(b"could not determine filesystem");
    }
    str_result(interp, b"native")
}

fn file_tempdir(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"file tempdir ?template?");
    }
    let template = argv
        .get(2)
        .map(|a| obj_bytes(*a))
        .unwrap_or_else(|| b"tcl".to_vec());
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    let path = temporary_path(interp, &template, "tcl", false);
    match fs.create_dir(&path) {
        Ok(()) => str_result(interp, path.as_bytes()),
        Err(e) => {
            let mut message = b"can't create temporary directory: ".to_vec();
            message.extend_from_slice(e.reason().as_bytes());
            interp.set_error(&message)
        }
    }
}

fn file_tempfile(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 4 {
        return interp.wrong_args(b"file tempfile ?nameVar? ?template?");
    }
    let var = argv.get(2).map(|a| obj_bytes(*a));
    let template = argv
        .get(3)
        .map(|a| obj_bytes(*a))
        .unwrap_or_else(|| b"tcl".to_vec());
    let path = temporary_path(interp, &template, "tcl", true);
    let host = interp.host();
    let Some(fs) = host.filesystem() else {
        return interp.set_error(b"filesystem not available");
    };
    if let Err(e) = fs.write(&path, &[]) {
        return host_fs_error(
            interp,
            b"couldn't create temporary file",
            path.as_bytes(),
            &e.reason(),
        );
    }
    if let Some(var) = var {
        let obj = new_string(path.as_bytes());
        if interp.var_set(&var, obj).is_err() {
            crate::interp::drop_fresh(obj);
            return interp.set_error(b"can't set temporary file name");
        }
    }
    // Native channels are backed by the same path; a VFS host can still use
    // the created path for subsequent file queries even if streaming is absent.
    let opened = interp.channels.borrow_mut().open(&path, b"w+");
    match opened {
        Ok(id) => {
            interp.set_result_bytes(&id);
            Code::Ok
        }
        Err(_) => str_result(interp, path.as_bytes()),
    }
}

/// Resolve Tcl's temporary-file template into a fresh-looking path. A
/// directory component is honoured verbatim; a bare template lives under the
/// host temporary directory. For files, the random part precedes the suffix
/// (`foo.bar` becomes `foo_<nonce>.bar`), matching `mkstemps`.
fn temporary_path(
    interp: &Interp,
    template: &[u8],
    default: &str,
    preserve_suffix: bool,
) -> String {
    let text = as_str(template);
    let (parent, leaf) = if text.ends_with('/') {
        (text.trim_end_matches('/').to_owned(), default)
    } else if let Some(slash) = text.rfind('/') {
        let parent = if slash == 0 { "/" } else { &text[..slash] };
        let leaf = text
            .get(slash + 1..)
            .filter(|s| !s.is_empty())
            .unwrap_or(default);
        (parent.to_owned(), leaf)
    } else {
        let parent = interp
            .host()
            .env()
            .get("TMPDIR")
            .or_else(|| interp.host().env().get("TEMP"))
            .or_else(|| interp.host().env().get("TMP"))
            .unwrap_or_else(|| "/tmp".to_owned());
        (parent, if text.is_empty() { default } else { text })
    };
    let (stem, suffix) = if preserve_suffix {
        let ext = tcl_cmd_core::path::extension(leaf.as_bytes());
        (&leaf[..leaf.len() - ext.len()], as_str(ext))
    } else {
        (leaf, "")
    };
    let nonce = interp.host().clock().now_millis();
    for attempt in 0..u16::MAX {
        let disambiguator = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let candidate = format!(
            "{}/{stem}_{nonce}{disambiguator}{suffix}",
            parent.trim_end_matches('/')
        );
        if !fs_exists(interp, candidate.as_bytes()) {
            return candidate;
        }
    }
    unreachable!("temporary path nonce space exhausted")
}

fn file_tildeexpand(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(path) = one_path(interp, argv, b"file tildeexpand name") else {
        return Code::Error;
    };
    if !path.starts_with(b"~") {
        return str_result(interp, &path);
    }
    let Some(home) = interp.host().env().get("HOME") else {
        return interp.set_error(b"could not determine home directory");
    };
    if path == b"~" {
        return str_result(interp, home.as_bytes());
    }
    if path.starts_with(b"~/") {
        return str_result(
            interp,
            format!("{}{}", home, &as_str(&path)[1..]).as_bytes(),
        );
    }
    interp.set_error(b"could not determine home directory")
}

fn file_volumes(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"file volumes");
    }
    if interp.host().filesystem().is_none() {
        return interp.set_error(b"filesystem not available");
    }
    let root = new_string(b"/");
    interp.set_result(list::new_list_obj(&[root]));
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
    use super::file_permissions;
    use crate::interp::{Code, Interp};
    use tcl_dialect::TclVersion;

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}: {}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
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
    fn file_surface_and_permissions_follow_the_runtime_profile() {
        let mut i = Interp::new();
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(i.eval_str(b"file tempdir"), Code::Error);
        let old_error = i.result_bytes();
        assert_eq!(
            old_error
                .windows(b"tempdir".len())
                .filter(|w| *w == b"tempdir")
                .count(),
            1,
            "8.6 visible subcommands must exclude tempdir: {}",
            String::from_utf8_lossy(&old_error)
        );
        assert_eq!(file_permissions(&i, 0o100755), b"0100755");

        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(file_permissions(&i, 0o100755), b"0o100755");
    }

    #[test]
    fn tempdir_template_honours_its_parent_and_requires_it_to_exist() {
        let mut i = Interp::new();
        let root = std::env::temp_dir().join(format!("tclrt_tempdir_{}", std::process::id()));
        let missing = root.join("missing").join("prefix");
        assert_eq!(
            i.eval_str(format!("file tempdir {}/", missing.display()).as_bytes()),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"can't create temporary directory: no such file or directory"
        );

        std::fs::create_dir_all(&root).unwrap();
        let created = ok(
            &mut i,
            format!("file tempdir {}/gorp", root.display()).as_bytes(),
        );
        let created = std::path::PathBuf::from(String::from_utf8(created).unwrap());
        assert_eq!(created.parent(), Some(root.as_path()));
        assert!(created
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("gorp_"));
        std::fs::remove_dir(&created).unwrap();
        std::fs::remove_dir(&root).unwrap();
    }

    #[test]
    fn file_extended_subcommands_cover_native_host_surface() {
        let mut i = Interp::new();
        // These pure arms otherwise default a missing operand to an empty
        // path; the runtime arity guard must reject them like Tcl does.
        assert_eq!(i.eval_str(b"file join"), Code::Error);
        assert_eq!(i.eval_str(b"file dirname"), Code::Error);
        let root = std::env::temp_dir().join(format!("tclrt_file_{}", std::process::id()));
        let src = root.join("src.txt");
        let copy = root.join("copy.txt");
        let moved = root.join("moved.txt");
        let src2 = root.join("src2.txt");
        let copied_dir = root.join("copied");
        let moved_dir = root.join("moved_dir");
        let link = root.join("link.txt");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&src, b"hello").unwrap();
        std::fs::write(&src2, b"world").unwrap();
        let s = |p: &std::path::Path| p.to_string_lossy().into_owned();
        let src_s = s(&src);
        let copy_s = s(&copy);
        let moved_s = s(&moved);
        let link_s = s(&link);
        assert_eq!(
            ok(&mut i, format!("file system {src_s}").as_bytes()),
            b"native"
        );
        assert_eq!(ok(&mut i, format!("file owned {src_s}").as_bytes()), b"1");
        assert!(
            String::from_utf8(ok(&mut i, format!("file atime {src_s}").as_bytes()))
                .unwrap()
                .parse::<i64>()
                .is_ok()
        );
        assert!(
            String::from_utf8(ok(&mut i, format!("file mtime {src_s}").as_bytes()))
                .unwrap()
                .parse::<i64>()
                .is_ok()
        );
        assert_eq!(
            ok(&mut i, format!("file attributes {src_s}").as_bytes()).contains(&b'-'),
            true
        );
        assert_eq!(
            ok(&mut i, format!("file copy {src_s} {copy_s}").as_bytes()),
            b""
        );
        assert_eq!(
            ok(&mut i, format!("file rename {copy_s} {moved_s}").as_bytes()),
            b""
        );
        ok(
            &mut i,
            format!("file mkdir {}", copied_dir.display()).as_bytes(),
        );
        ok(
            &mut i,
            format!(
                "file copy {src_s} {} {}",
                src2.display(),
                copied_dir.display()
            )
            .as_bytes(),
        );
        assert_eq!(
            ok(
                &mut i,
                format!("file exists {}/src2.txt", copied_dir.display()).as_bytes()
            ),
            b"1"
        );
        ok(
            &mut i,
            format!("file mkdir {}", moved_dir.display()).as_bytes(),
        );
        ok(
            &mut i,
            format!(
                "file rename {}/src.txt {}/",
                copied_dir.display(),
                moved_dir.display()
            )
            .as_bytes(),
        );
        assert_eq!(
            ok(
                &mut i,
                format!("file exists {}/src.txt", moved_dir.display()).as_bytes()
            ),
            b"1"
        );
        assert_eq!(
            ok(
                &mut i,
                format!("file link -symbolic {link_s} {src_s}").as_bytes()
            ),
            src_s.as_bytes()
        );
        assert_eq!(
            ok(&mut i, format!("file readlink {link_s}").as_bytes()),
            src_s.as_bytes()
        );
        assert_eq!(
            ok(
                &mut i,
                format!("file stat {src_s} st; set st(size)").as_bytes()
            ),
            b"5"
        );
        assert_eq!(
            ok(
                &mut i,
                format!("file lstat {link_s} lst; set lst(type)").as_bytes()
            ),
            b"link"
        );
        assert_eq!(ok(&mut i, b"file tildeexpand plain"), b"plain");
        let home = ok(&mut i, b"file home");
        assert!(!home.is_empty());
        let tempdir = ok(&mut i, b"file tempdir tclrt-test");
        assert_eq!(
            ok(
                &mut i,
                format!("file isdirectory {}", String::from_utf8_lossy(&tempdir)).as_bytes()
            ),
            b"1"
        );
        let tempfile = ok(&mut i, b"file tempfile tmpname tclrt-test");
        assert!(String::from_utf8_lossy(&tempfile).starts_with("file"));
        assert!(!ok(&mut i, b"file channels").is_empty());
        ok(
            &mut i,
            format!("close {}", String::from_utf8_lossy(&tempfile)).as_bytes(),
        );
        assert!(!ok(&mut i, b"file volumes").is_empty());
        ok(
            &mut i,
            format!("file delete -force {}", root.display()).as_bytes(),
        );
        ok(
            &mut i,
            format!("file delete -force {}", String::from_utf8_lossy(&tempdir)).as_bytes(),
        );
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
