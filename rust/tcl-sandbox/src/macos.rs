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

//! The macOS OS-native tier: Seatbelt, the kernel sandbox `sandbox-exec`
//! applies to the program it runs.
//!
//! The profile is written from the grant: everything allowed by default, then
//! the network denied but for local (Unix-domain) sockets, writes denied
//! outside the write paths and the standard devices, and reading file contents
//! denied outside the read and write paths, the system trees, the directories
//! on the `PATH` the child is given and the program's own installation —
//! metadata stays readable, so path lookups through an ungranted directory
//! still resolve. Paths reach the profile as `-D` parameters, never spliced
//! into its text. Seatbelt matches resolved paths, so each path is granted as
//! named and as resolved (`/tmp` is `/private/tmp`).

use std::fmt::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use crate::{Confinement, FsGrant, Grant, IsolationLevel, Profile};

/// The wrapper that applies a profile to the program it runs.
const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// The system trees every program needs to read to load and run.
const SYSTEM_READ: &[&str] = &[
    "/System",
    "/usr",
    "/bin",
    "/sbin",
    "/Library",
    "/private/etc",
    "/private/var/db",
    "/private/var/select",
    "/dev",
    "/opt/homebrew",
    "/opt/local",
    "/nix/store",
];

/// The devices a program may write.
const DEVICES: &[&str] = &[
    "/dev/null",
    "/dev/zero",
    "/dev/tty",
    "/dev/dtracehelper",
    "/dev/random",
    "/dev/urandom",
];

/// Seatbelt through `sandbox-exec`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Seatbelt;

impl Seatbelt {
    /// The tier, when `sandbox-exec` is present and applies a trivial profile.
    /// Probed once per process.
    pub(crate) fn detect() -> Option<Self> {
        static PROBED: OnceLock<bool> = OnceLock::new();
        PROBED
            .get_or_init(|| {
                Command::new(SANDBOX_EXEC)
                    .args(["-p", "(version 1)(allow default)", "/usr/bin/true"])
                    .env_clear()
                    .status()
                    .is_ok_and(|status| status.success())
            })
            .then_some(Self)
    }
}

impl Confinement for Seatbelt {
    fn level(&self) -> IsolationLevel {
        IsolationLevel::Os
    }

    fn enforces_network_deny(&self, _grant: &Grant) -> bool {
        true
    }

    fn enforces_filesystem(&self, grant: &Grant) -> bool {
        grant.filesystem.is_some()
    }

    fn command(&self, profile: &Profile, grant: &Grant) -> Command {
        if grant.network && grant.filesystem.is_none() {
            let mut cmd = Command::new(&profile.program);
            cmd.args(&profile.args);
            return cmd;
        }
        let mut rules = vec!["(version 1)".to_owned(), "(allow default)".to_owned()];
        let mut params: Vec<(String, PathBuf)> = Vec::new();
        if !grant.network {
            rules.push("(deny network*)".to_owned());
            rules.push("(allow network* (remote unix-socket))".to_owned());
        }
        if let Some(fs) = &grant.filesystem {
            let fs = with_system_paths(fs, profile);
            let write = filters("W", &fs.write, &mut params);
            let read = filters("R", fs.read.iter().chain(&fs.write), &mut params);
            let devices = DEVICES.iter().fold(String::new(), |mut out, device| {
                let _ = write!(out, "(literal \"{device}\")");
                out
            });
            rules.push(format!(
                "(deny file-write* (require-not (require-any {write}{devices})))"
            ));
            rules.push(format!(
                "(deny file-read-data (require-not (require-any {read}{devices})))"
            ));
        }
        let mut cmd = Command::new(SANDBOX_EXEC);
        cmd.arg("-p").arg(rules.concat());
        for (name, path) in &params {
            cmd.arg("-D").arg(format!("{name}={}", path.display()));
        }
        cmd.arg(&profile.program).args(&profile.args);
        cmd
    }
}

/// A `subpath` filter for each of `paths`, as named and as resolved, each
/// path passed as a parameter named `prefix` and its index.
fn filters<'a>(
    prefix: &str,
    paths: impl IntoIterator<Item = &'a PathBuf>,
    params: &mut Vec<(String, PathBuf)>,
) -> String {
    let mut seen: Vec<PathBuf> = Vec::new();
    for path in paths {
        let resolved = std::fs::canonicalize(path).ok();
        for path in std::iter::once(path.clone()).chain(resolved) {
            if path.is_absolute() && !seen.contains(&path) {
                seen.push(path);
            }
        }
    }
    seen.into_iter()
        .map(|path| {
            let name = format!("{prefix}{}", params.len());
            let filter = format!("(subpath (param \"{name}\"))");
            params.push((name, path));
            filter
        })
        .collect()
}

/// `grant` widened by what any program needs to start: the system trees, the
/// directories on the `PATH` it is given, and the program's own installation —
/// its directory and the `lib` and `share` beside it, as named and as resolved.
fn with_system_paths(grant: &FsGrant, profile: &Profile) -> FsGrant {
    let mut fs = grant.clone();
    fs.read.extend(SYSTEM_READ.iter().map(PathBuf::from));
    let path = profile
        .env_set
        .iter()
        .find(|(name, _)| name == "PATH")
        .map(|(_, value)| value.clone().into())
        .or_else(|| std::env::var_os("PATH"));
    fs.read.extend(
        path.iter()
            .flat_map(std::env::split_paths)
            .filter(|dir| dir.is_absolute()),
    );
    let program = profile.program.as_path();
    let resolved = std::fs::canonicalize(program).ok();
    for program in std::iter::once(program).chain(resolved.as_deref()) {
        let Some(dir) = program.parent().filter(|dir| dir.is_absolute()) else {
            continue;
        };
        fs.read.push(dir.to_path_buf());
        if let Some(prefix) = dir.parent() {
            fs.read.push(prefix.join("lib"));
            fs.read.push(prefix.join("share"));
        }
    }
    fs.read.retain(|path| path.exists());
    fs
}
