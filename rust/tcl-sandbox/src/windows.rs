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

//! The Windows OS-native tier: an app container, through `rappct`, which holds
//! the Win32 calls.
//!
//! A process in an app container reaches only what the system grants every
//! application package (the Windows and program directories) and what is
//! granted to its own package SID; it has no network unless it holds a
//! network capability. So the grant's paths are granted to the package — read
//! and execute beneath the read paths and the program's installation, full
//! access beneath the write paths — and the child holds `internetClient` and
//! `privateNetworkClientServer` only when it may use the network. The grants
//! are access-control entries for the one `tcl-lsp.sandbox` package: they
//! persist on the granted directories, and admit no process outside that
//! container. The child runs in a Job Object that kills the whole tree when
//! the run ends or times out.
//!
//! A profile that names no path runs as an ordinary process: an app container
//! cannot leave the filesystem open, so its network is not denied either.

use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use rappct::acl::{AccessMask, ResourcePath, grant_to_package};
use rappct::{
    AcError, AppContainerProfile, AppContainerSid, JobLimits, KnownCapability, LaunchOptions,
    LaunchedIo, SecurityCapabilitiesBuilder, StdioConfig, launch_in_container_with_io,
};

use crate::{Confinement, Ended, Grant, IsolationLevel, Process, Profile, Spawned};

/// The app container profile every sandboxed child runs as.
const PACKAGE: &str = "tcl-lsp.sandbox";

/// `FILE_GENERIC_EXECUTE`, which `rappct` does not name.
const FILE_GENERIC_EXECUTE: u32 = 0x0012_00A0;

/// The app container tier, with the package every child runs as.
#[derive(Debug, Clone)]
pub(crate) struct AppContainer {
    package: AppContainerSid,
}

impl AppContainer {
    /// The tier, when this Windows can create (or open) the package. Probed
    /// once per process.
    pub(crate) fn detect() -> Option<Self> {
        static PROBED: OnceLock<Option<AppContainerSid>> = OnceLock::new();
        PROBED
            .get_or_init(|| {
                AppContainerProfile::ensure(
                    PACKAGE,
                    "tcl-lsp sandbox",
                    Some("Confines the processes tcl-lsp's package manager runs"),
                )
                .ok()
                .map(|profile| profile.sid)
            })
            .clone()
            .map(|package| Self { package })
    }

    /// Grant the package access beneath `path`, as a directory or a file.
    fn grant(&self, path: &Path, access: AccessMask) -> rappct::Result<()> {
        let target = if path.is_dir() {
            ResourcePath::Directory(path.to_path_buf())
        } else {
            ResourcePath::File(path.to_path_buf())
        };
        grant_to_package(target, &self.package, access)
    }
}

impl Confinement for AppContainer {
    fn level(&self) -> IsolationLevel {
        IsolationLevel::Os
    }

    fn enforces_network_deny(&self, grant: &Grant) -> bool {
        grant.filesystem.is_some()
    }

    fn enforces_filesystem(&self, grant: &Grant) -> bool {
        grant.filesystem.is_some()
    }

    fn spawn(&self, cmd: &mut Command, profile: &Profile, grant: &Grant) -> io::Result<Spawned> {
        let Some(fs) = &grant.filesystem else {
            return cmd.spawn().map(Spawned::from);
        };
        let program = PathBuf::from(cmd.get_program());
        let read = AccessMask(AccessMask::FILE_GENERIC_READ.0 | FILE_GENERIC_EXECUTE);
        // A read path the package cannot be granted (one the user does not
        // own) is often readable by every package already, and the child
        // fails on it if not; a write path it cannot be granted fails the run.
        for path in fs.read.iter().chain(&installation(&program)) {
            if path.exists() {
                let _ = self.grant(path, read);
            }
        }
        for path in fs.write.iter().filter(|path| path.exists()) {
            self.grant(path, AccessMask::GENERIC_ALL)
                .map_err(io::Error::other)?;
        }
        let mut capabilities = SecurityCapabilitiesBuilder::new(&self.package);
        if grant.network {
            capabilities = capabilities.with_known(&[
                KnownCapability::InternetClient,
                KnownCapability::PrivateNetworkClientServer,
            ]);
        }
        let capabilities = capabilities.build().map_err(io::Error::other)?;
        let options = LaunchOptions {
            cmdline: Some(command_line(cmd.get_program(), cmd.get_args())),
            exe: program,
            cwd: cmd.get_current_dir().map(Path::to_path_buf),
            env: Some(
                cmd.get_envs()
                    .filter_map(|(name, value)| Some((name.to_owned(), value?.to_owned())))
                    .collect(),
            ),
            // The streams are piped together or inherited together: a child
            // fed on its standard input has its output captured with it.
            stdio: if profile.capture || profile.stdin_data.is_some() {
                StdioConfig::Pipe
            } else {
                StdioConfig::Inherit
            },
            suspended: false,
            join_job: Some(JobLimits {
                kill_on_job_close: true,
                ..JobLimits::default()
            }),
            startup_timeout: None,
        };
        launch_in_container_with_io(&capabilities, &options)
            .map(|launched| Spawned(Process::Container(Container(launched))))
            .map_err(io::Error::other)
    }
}

/// The program's installation: its directory and the `lib` and `share`
/// beside it.
fn installation(program: &Path) -> Vec<PathBuf> {
    let Some(dir) = program.parent().filter(|dir| dir.is_absolute()) else {
        return Vec::new();
    };
    let mut paths = vec![dir.to_path_buf()];
    if let Some(prefix) = dir.parent() {
        paths.push(prefix.join("lib"));
        paths.push(prefix.join("share"));
    }
    paths
}

/// The command line `CreateProcess` parses back into `program` and `args`
/// (the Microsoft C runtime's rules).
fn command_line<'a>(program: &'a OsStr, args: impl Iterator<Item = &'a OsStr>) -> String {
    std::iter::once(program)
        .chain(args)
        .map(quote)
        .collect::<Vec<_>>()
        .join(" ")
}

/// One argument quoted for [`command_line`]: as it is when nothing in it needs
/// quoting, else in quotes, with each run of backslashes before a quote (or
/// the closing quote) doubled and each quote escaped.
fn quote(arg: &OsStr) -> String {
    let arg = arg.to_string_lossy();
    if !arg.is_empty() && !arg.contains([' ', '\t', '\n', '\u{b}', '"']) {
        return arg.into_owned();
    }
    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(backslashes));
                quoted.push(c);
                backslashes = 0;
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

/// A child running in the app container.
pub(crate) struct Container(LaunchedIo);

impl Container {
    pub(crate) fn take_stdin(&mut self) -> Option<Box<dyn Write + Send>> {
        self.0
            .stdin
            .take()
            .map(|sink| Box::new(sink) as Box<dyn Write + Send>)
    }

    pub(crate) fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.0
            .stdout
            .take()
            .map(|source| Box::new(source) as Box<dyn Read + Send>)
    }

    pub(crate) fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>> {
        self.0
            .stderr
            .take()
            .map(|source| Box::new(source) as Box<dyn Read + Send>)
    }

    /// Wait for the child. A wait that times out drops the job, which kills
    /// the tree.
    pub(crate) fn wait(self, timeout: Option<Duration>) -> io::Result<Ended> {
        match self.0.wait(timeout) {
            Ok(code) => Ok(Ended {
                code: Some(i32::from_ne_bytes(code.to_ne_bytes())),
                success: code == 0,
                timed_out: false,
            }),
            Err(AcError::LaunchFailed {
                stage: "wait",
                hint: "timeout",
                ..
            }) => Ok(Ended {
                code: None,
                success: false,
                timed_out: true,
            }),
            Err(error) => Err(io::Error::other(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_quoted_as_the_c_runtime_parses_them() {
        let line = command_line(
            OsStr::new(r"C:\Tcl\bin\tclsh.exe"),
            [
                "plain",
                "two words",
                "",
                r#"say "hi""#,
                r"trailing\",
                r"C:\dir with space\",
            ]
            .iter()
            .map(OsStr::new),
        );
        assert_eq!(
            line,
            r#"C:\Tcl\bin\tclsh.exe plain "two words" "" "say \"hi\"" trailing\ "C:\dir with space\\""#
        );
    }
}
