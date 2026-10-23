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

//! Platform-backed command helpers (`exec`, `pwd`, `cd`, …).
//!
//! The portable Tcl semantics (argument parsing, the error catalogue, result
//! building) live here once, while the actual host syscalls are reached through
//! the [`tcl_platform`] capability traits and live per-target. A
//! **platform-conditional** capability (subprocess, sockets) that
//! a host lacks — every WASM target lacks subprocess — surfaces as the faithful
//! Tcl "unsupported" error rather than a panic, exactly as the capability model
//! intends.

use tcl_platform::Host;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use crate::path::PathReading;

/// Installation directories appended to `auto_path`, in Tcl lookup order,
/// with paths read as `reading`'s release reads them.
#[must_use]
pub fn auto_path_installation(
    library: Option<&[u8]>,
    executable: Option<&[u8]>,
    reading: PathReading,
) -> Vec<Vec<u8>> {
    let mut paths = Vec::new();
    if let Some(library) = library {
        paths.push(library.to_vec());
        paths.push(crate::path::dirname(library, reading).into_owned());
    }
    if let Some(executable) = executable {
        let directory = crate::path::dirname(executable, reading);
        let parent = crate::path::dirname(&directory, reading);
        paths.push(crate::path::join(&[parent.as_ref(), b"lib"], reading));
    }
    paths
}

/// Expand the supplied host home directory without guessing named user homes;
/// joined paths are read as `reading`'s release reads them.
pub fn auto_path_environment(
    paths: Vec<Vec<u8>>,
    home: Option<&[u8]>,
    reading: PathReading,
) -> Result<Vec<Vec<u8>>, CmdError> {
    let mut expanded = Vec::new();
    for path in paths {
        let selected = {
            if path == b"~" {
                home.map(<[u8]>::to_vec)
            } else if path.starts_with(b"~/") {
                home.map(|home| crate::path::join(&[home, &path[2..]], reading))
            } else if path.starts_with(b"~") {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "named home directory lookup",
                )
                .into());
            } else {
                Some(path)
            }
        };
        if let Some(path) = selected {
            expanded.push(path);
        }
    }
    Ok(expanded)
}

/// `exec arg ?arg ...?` — run a command and capture its standard output (with
/// trailing newlines trimmed).
///
/// Only the minimal form is handled: every argument is passed through as a word
/// of the command line, so redirections, pipelines and `-keepnewline` have no
/// effect here.
///
/// Subprocess execution is a *platform-conditional* capability: when the host
/// provides no [`Process`](tcl_platform::Process) (every WASM target, a sandbox),
/// this yields the faithful unsupported error instead of running — the same
/// shared body, a different host.
pub fn exec<O: ValueOps>(
    ops: &mut O,
    host: &dyn Host,
    args: &[O::Value],
) -> Result<O::Value, CmdError> {
    let Some(process) = host.process() else {
        return Err(CmdError::new(
            "exec is not available: this platform provides no subprocess support",
        ));
    };
    if args.is_empty() {
        return Err(CmdError::wrong_args("exec ?-option ...? arg ?arg ...?"));
    }
    let parts: Vec<String> = args
        .iter()
        .map(|a| ops.try_as_str(a).map(|text| text.to_string()))
        .collect::<Result<_, _>>()?;
    let part_refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    let output = process.run(&part_refs)?;
    let mut out = output.stdout;
    while out.last() == Some(&b'\n') {
        out.pop();
    }
    Ok(ops.new_bytes(&out))
}

/// `pwd` — the current working directory. The working directory is a *mandatory*
/// capability ([`Env`](tcl_platform::Env)), virtualised on hosts without a real
/// one, so this never reports "unsupported".
pub fn pwd<O: ValueOps>(ops: &mut O, host: &dyn Host) -> Result<O::Value, CmdError> {
    let dir = host.env().cwd()?;
    Ok(ops.new_string(dir))
}
