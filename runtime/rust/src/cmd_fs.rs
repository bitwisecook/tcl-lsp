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

//! Filesystem commands — `source`, `file`, `glob`, `pwd`, `cd`.
//!
//! All filesystem and working-directory access goes through the capability host
//! ([`Interp::host`](crate::interp::Interp)) — the [`tcl_platform`]
//! `Filesystem`/`Env` traits — rather than direct `std::fs`/`std::env`. A native
//! build gets the std-backed `NativeHost`; the WASM targets get a restricted
//! host (a no-VFS browser answers `false`/"unsupported"). C refs:
//! `tclIOUtil.c`/`tclFileName.c` (`source`/`glob`), `tclFCmd.c`/`tclFileName.c`
//! (`file`). This groundwork is required to load the real
//! `init.tcl`/`tcltest.tcl`.
//!
//! Path handling is `/`-separated (Tcl's portable convention); fine on Unix /
//! WASI.

use std::borrow::Cow;

use tcl_cmd_core::error::CmdError;
use tcl_cmd_core::path::{self, PathReading};
use tcl_dialect::model::{surface_admits, SurfaceQuery};
use tcl_platform::{Filesystem, HostError};

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `source`, `file`, `glob`, `pwd`, `cd`.
pub fn install(interp: &mut Interp) {
    interp.register_stock_builtin(b"source", source_cmd);
    install_file(interp);
    interp.register_stock_builtin(b"glob", glob_cmd);
    interp.register_stock_builtin(b"pwd", pwd_cmd);
    interp.register_stock_builtin(b"cd", cd_cmd);
}

pub(crate) fn install_file(interp: &mut Interp) {
    const NAMES: &[&[u8]] = &[
        b"atime",
        b"attributes",
        b"channels",
        b"copy",
        b"delete",
        b"dirname",
        b"executable",
        b"exists",
        b"extension",
        b"home",
        b"isdirectory",
        b"isfile",
        b"join",
        b"link",
        b"lstat",
        b"mkdir",
        b"mtime",
        b"nativename",
        b"normalize",
        b"owned",
        b"pathtype",
        b"readable",
        b"readlink",
        b"rename",
        b"rootname",
        b"separator",
        b"size",
        b"split",
        b"stat",
        b"system",
        b"tail",
        b"tempdir",
        b"tempfile",
        b"tildeexpand",
        b"type",
        b"volumes",
        b"writable",
    ];
    let admitted = crate::environment::release_subcommands(
        interp.native_ensemble_profile_name(),
        "file",
        NAMES,
    );
    interp.register_stock_ensemble(
        tcl_registry::invocation_words::EnsembleImplementationFamily::File,
        b"file",
        file_cmd,
        FILE_MEMBERS,
        admitted,
    );
}

macro_rules! file_members {
    ($($function:ident => $member:literal),+ $(,)?) => {
        const FILE_MEMBERS: &[(&[u8], crate::interp::BuiltinFn)] = &[
            $(($member, $function)),+
        ];
        $(fn $function(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
            interp.invoke_stock_worker(argv, &[b"file", $member], file_cmd)
        })+
    };
}

file_members! {
    stock_file_atime => b"atime",
    stock_file_attributes => b"attributes",
    stock_file_channels => b"channels",
    stock_file_copy => b"copy",
    stock_file_delete => b"delete",
    stock_file_dirname => b"dirname",
    stock_file_executable => b"executable",
    stock_file_exists => b"exists",
    stock_file_extension => b"extension",
    stock_file_home => b"home",
    stock_file_isdirectory => b"isdirectory",
    stock_file_isfile => b"isfile",
    stock_file_join => b"join",
    stock_file_link => b"link",
    stock_file_lstat => b"lstat",
    stock_file_mkdir => b"mkdir",
    stock_file_mtime => b"mtime",
    stock_file_nativename => b"nativename",
    stock_file_normalize => b"normalize",
    stock_file_owned => b"owned",
    stock_file_pathtype => b"pathtype",
    stock_file_readable => b"readable",
    stock_file_readlink => b"readlink",
    stock_file_rename => b"rename",
    stock_file_rootname => b"rootname",
    stock_file_separator => b"separator",
    stock_file_size => b"size",
    stock_file_split => b"split",
    stock_file_stat => b"stat",
    stock_file_system => b"system",
    stock_file_tail => b"tail",
    stock_file_tempdir => b"tempdir",
    stock_file_tempfile => b"tempfile",
    stock_file_tildeexpand => b"tildeexpand",
    stock_file_type => b"type",
    stock_file_volumes => b"volumes",
    stock_file_writable => b"writable",
}

fn as_str(b: &[u8]) -> &str {
    core::str::from_utf8(b).unwrap_or("")
}

// source

/// Read a file after the actual native source grammar selects its operands.
fn source_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use crate::interp::{
        native_arguments::OriginalArguments, native_operation_currency::NativeOperationScope,
    };
    let operation = match NativeOperationScope::enter(interp) {
        Ok(operation) => operation,
        Err(error) => {
            return interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(error));
        }
    };
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let reached = source_in_scope(interp, argv, &arguments);
    let reached = arguments.finish(reached);
    drop(arguments);
    if operation.currency().ensure_current_or_refuse().is_err() {
        return Code::Error;
    }
    reached
}

fn source_in_scope(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &crate::interp::native_arguments::OriginalArguments,
) -> Code {
    use tcl_registry::source_file::{SourceFileGrammar, SourceFileSelection};
    let dialect = interp.native_invocation_dialect();
    let Some(grammar) = dialect.source_file_grammar() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("native source grammar")
                .into(),
        );
    };
    let Some(protocol) = dialect.native_source_file_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native source file protocol",
            )
            .into(),
        );
    };
    let count = argv.len().saturating_sub(1);
    let selector = if count > 1 && grammar != SourceFileGrammar::Filename {
        match arguments.bytes(1) {
            Ok(bytes) => Some(bytes),
            Err(error) => return interp.report_cmd_error(error),
        }
    } else {
        None
    };
    let SourceFileSelection::Selected(selected) =
        grammar.select_original(count, selector.as_deref())
    else {
        return interp.wrong_args_for_invocation(argv, grammar.usage_suffix().as_bytes());
    };
    let prepared = tcl_cmd_core::encoding::prepare_source_file(
        &mut crate::interp::native_encoding_source::SourceOps {
            interpreter: interp,
            arguments,
            encoding_at: selected.encoding_at,
            reached: Vec::new(),
        },
        protocol,
        selected.path_at,
        selected.encoding_at,
    );
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return interp.report_cmd_error(arguments.retain_refusal(error)),
    };
    if let Err(error) = arguments.check_for(interp) {
        return interp.report_cmd_error(error);
    }
    let package_scope = selected
        .no_package
        .then(|| interp.take_package_file_scope());
    let code = interp.eval_sourced(&prepared.script, &prepared.path);
    if let Some(scope) = package_scope {
        interp.restore_package_file_scope(scope);
    }
    arguments.finish(code)
}

// file

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
    // on its document authoring mask.
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
        // The `/`-based path text ops are the shared `tcl_cmd_core::path` core,
        // read as the emulated release reads a leading `//` and a `~`.
        b"dirname" | b"tail" => {
            let name = arg(2).unwrap_or_default();
            let release = interp.runtime_version();
            let host = interp.host();
            let part = path::part_name(&name, release, host.env(), host.filesystem()).map(|part| {
                let reading = PathReading::of_release(release);
                if sub.as_slice() == b"dirname" {
                    path::dirname(&part, reading).into_owned()
                } else {
                    path::tail(&part, reading).into_owned()
                }
            });
            path_result(interp, part)
        }
        b"rootname" => str_result(interp, path::rootname(&arg(2).unwrap_or_default())),
        b"extension" => str_result(interp, path::extension(&arg(2).unwrap_or_default())),
        b"home" => file_home(interp, argv),
        b"join" => {
            let parts: Vec<Vec<u8>> = argv[2..].iter().map(|&a| obj_bytes(a)).collect();
            str_result(interp, &path::join(&parts, reading(interp)))
        }
        b"link" => file_link(interp, argv),
        b"lstat" => file_stat(interp, argv, true),
        b"split" => {
            let name = arg(2).unwrap_or_default();
            let elems: Vec<*mut TclObj> = path::split(&name, reading(interp))
                .iter()
                .map(|element| new_string(element))
                .collect();
            interp.set_result(interp.new_list_object(&elems));
            Code::Ok
        }
        b"normalize" => {
            let release = interp.runtime_version();
            let host = interp.host();
            let normal = path::normalize(
                &arg(2).unwrap_or_default(),
                release,
                host.env(),
                host.filesystem(),
            );
            path_result(interp, normal)
        }
        b"owned" => file_owned(interp, argv),
        b"separator" => str_result(interp, b"/"),
        b"nativename" => {
            let name = arg(2).unwrap_or_default();
            let release = interp.runtime_version();
            let host = interp.host();
            let native = path::translate(&name, release, host.env()).map(Cow::into_owned);
            path_result(interp, native)
        }
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
            let kind = path::path_type(&arg(2).unwrap_or_default(), reading(interp));
            str_result(interp, kind.name().as_bytes())
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
    // The join — including the ensemble's comma before `or`, even for two
    // entries — belongs to `tcl_cmd_core::ensemble`, not to this module.
    interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
        &visible,
        raw,
        true,
        b"::tcl::file",
    ))
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

/// How the emulated release reads a leading `//` and a `~` in a path.
fn reading(interp: &Interp) -> PathReading {
    PathReading::of_release(interp.runtime_version())
}

/// Where `file copy` and `file rename` put `source` in the directory
/// `target`: `target` joined with the source's tail as `file tail` reads it.
fn into_directory(interp: &Interp, target: &[u8], source: &[u8]) -> Result<Vec<u8>, CmdError> {
    let release = interp.runtime_version();
    let reading = PathReading::of_release(release);
    let host = interp.host();
    let part = path::part_name(source, release, host.env(), host.filesystem())?;
    Ok(path::join(&[target, &path::tail(&part, reading)], reading))
}

/// A path the shared core built, or the error it raised.
fn path_result(interp: &mut Interp, path: Result<Vec<u8>, CmdError>) -> Code {
    match path {
        Ok(path) => str_result(interp, &path),
        Err(error) => interp.report_cmd_error(error),
    }
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
        let secs = match crate::typed_value::wide_int(*value) {
            Ok(secs) => secs,
            Err(error) => return interp.report_typed_error(&error),
        };
        return match fs.set_times(
            as_str(&path),
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
    match fs.metadata(as_str(&path)) {
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
    let meta = match fs.symlink_metadata(as_str(&path)) {
        Ok(m) => m,
        Err(e) => return host_fs_error(interp, b"could not read", &path, &e.reason()),
    };
    if argv.len() == 3 {
        // The entry's own name: a `~` in it names no home.
        let tail = path::tail(&path, PathReading::Tcl9).into_owned();
        let entries = [
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
        interp.set_result(interp.new_list_object(&objs));
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
    interp.set_result(interp.new_list_object(&objs));
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
    let target_is_dir = fs.symlink_metadata(as_str(&target)).is_ok_and(|m| m.is_dir);
    let sources = &argv[i..argv.len() - 1];
    if sources.len() > 1 && !target_is_dir {
        return interp.set_error(b"target must be a directory");
    }
    for &source_obj in sources {
        let source = obj_bytes(source_obj);
        let destination = if target_is_dir {
            match into_directory(interp, &target, &source) {
                Ok(destination) => destination,
                Err(error) => return path_result(interp, Err(error)),
            }
        } else {
            target.clone()
        };
        let recursive = fs.symlink_metadata(as_str(&source)).is_ok_and(|m| m.is_dir);
        if let Err(e) = fs.copy(as_str(&source), as_str(&destination), recursive, force) {
            return host_fs_error(interp, b"error copying", &source, &e.reason());
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn file_home(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"file home ?user?");
    }
    let user = argv.get(2).map(|&a| obj_bytes(a)).unwrap_or_default();
    let release = interp.runtime_version();
    let host = interp.host();
    let home = path::home_directory(host.env(), &user, release);
    path_result(interp, home)
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
    match fs.link(as_str(&link), as_str(&target), hard) {
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
    match fs.readlink(as_str(path)) {
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
    let target_is_dir = fs.symlink_metadata(as_str(&target)).is_ok_and(|m| m.is_dir);
    let sources = &argv[i..argv.len() - 1];
    if sources.len() > 1 && !target_is_dir {
        return interp.set_error(b"target must be a directory");
    }
    for &source_obj in sources {
        let source = obj_bytes(source_obj);
        let destination = if target_is_dir {
            match into_directory(interp, &target, &source) {
                Ok(destination) => destination,
                Err(error) => return path_result(interp, Err(error)),
            }
        } else {
            target.clone()
        };
        if let Err(e) = fs.rename(as_str(&source), as_str(&destination), force) {
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
        fs.symlink_metadata(as_str(&path))
    } else {
        fs.metadata(as_str(&path))
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
        interp.set_result(interp.new_list_object(&objs));
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
    let version = interp.runtime_version();
    let access = tcl_cmd_core::channel::resolve_open_access_mode(version, "w+")
        .expect("the literal w+ access mode is valid");
    let opened = interp.channels.borrow_mut().open(&path, access, version);
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
    let Some(name) = one_path(interp, argv, b"file tildeexpand path") else {
        return Code::Error;
    };
    let release = interp.runtime_version();
    let host = interp.host();
    let expanded = path::tilde_expand(&name, release, host.env()).map(Cow::into_owned);
    path_result(interp, expanded)
}

fn file_volumes(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"file volumes");
    }
    if interp.host().filesystem().is_none() {
        return interp.set_error(b"filesystem not available");
    }
    let root = new_string(b"/");
    interp.set_result(interp.new_list_object(&[root]));
    Code::Ok
}

// pwd / cd

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

// glob

/// `glob ?-nocomplain? ?-directory dir? ?-tails? ?-join? ?--? pattern ...` —
/// filesystem name matching.
fn glob_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let version = interp.runtime_version();
    let jim = interp.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Jim);
    let args = &argv[1..];
    let options =
        match tcl_cmd_core::filesystem_glob::GlobOptions::parse(interp, args, version, jim) {
            Ok(options) => options,
            Err(error) => return interp.report_cmd_error(error),
        };
    let directory = options.base.map(|base| obj_bytes(args[base.index()]));
    // Patterns: each remaining arg, or (with -join) all joined into one.
    let pats: Vec<Vec<u8>> = args[options.patterns_start..]
        .iter()
        .map(|&a| obj_bytes(a))
        .collect();
    let patterns: Vec<Vec<u8>> = if options.join {
        vec![path::join(&pats, reading(interp))]
    } else {
        pats
    };

    let base = directory.clone();
    let mut hits: Vec<Vec<u8>> = Vec::new();
    // The host filesystem drives directory listing + the `-types` stats. An
    // independent `Rc` handle (`host`) keeps the borrow off `interp`, which the
    // result/error tail still needs mutably.
    let host = interp.host();
    let fs = host.filesystem();
    if let (false, Some(fs)) = (jim, fs) {
        hits = match options.c_unix_matches(fs, base.as_deref(), &patterns, version) {
            Ok(hits) => hits,
            Err(error) => return interp.report_cmd_error(error),
        };
    } else {
        for pat in &patterns {
            glob_one(
                fs,
                base.as_deref(),
                pat,
                options.tails,
                &options.types,
                &mut hits,
            );
        }
        hits.sort();
        hits.dedup();
        if hits.is_empty() {
            if let Some(error) = options.no_match(&patterns, version, jim) {
                return interp.report_cmd_error(error);
            }
        }
    }
    let objs: Vec<*mut TclObj> = hits.iter().map(|h| new_string(h)).collect();
    interp.set_result(interp.new_list_object(&objs));
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
    types: &tcl_cmd_core::filesystem_glob::GlobTypes,
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
    types: &tcl_cmd_core::filesystem_glob::GlobTypes,
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
            // Apply file-kind and permission filters.
            if !fs.is_some_and(|fs| types.matches(fs, as_str(&child_path))) {
                continue;
            }
            hits.push(child_prefix);
        } else {
            walk(fs, &child_path, &child_prefix, segs, idx + 1, types, hits);
        }
    }
}

/// Whether a single path segment matches a glob segment. A literal segment (no
/// glob metacharacters) matches by equality; otherwise by `string match`.
fn glob_seg_match(pat: &[u8], name: &[u8]) -> bool {
    if pat.iter().any(|&c| matches!(c, b'*' | b'?' | b'[' | b'\\')) {
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
mod tests;
