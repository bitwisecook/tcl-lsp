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

//! The Linux OS-native tier: Landlock confines the filesystem to a grant and a
//! seccomp filter refuses the sockets that reach a network.
//!
//! Both restrict the calling thread and every process it forks, never the
//! rest of the process, so the child is spawned from a thread of its own that
//! takes on the restrictions first: the parent keeps its authority, the child
//! inherits the thread's, and nothing in this crate needs `unsafe` (the wrapper
//! crates hold the syscalls). Each mechanism is probed once, on a throwaway
//! thread, so a kernel without it is reported, not assumed.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use landlock::{
    ABI, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus,
    path_beneath_rules,
};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};

use crate::{Confinement, FsGrant, Grant, IsolationLevel, Profile, Spawned};

/// The Landlock ABI the rules are written against; an older kernel enforces
/// the subset it knows.
const LANDLOCK_ABI: ABI = ABI::V3;

/// The system trees every program needs to read to load and run.
const SYSTEM_READ: &[&str] = &[
    "/usr",
    "/lib",
    "/lib32",
    "/lib64",
    "/libx32",
    "/bin",
    "/sbin",
    "/etc",
    "/opt",
    "/nix/store",
];

/// The devices a program may read and write.
const DEVICES: &[&str] = &[
    "/dev/null",
    "/dev/zero",
    "/dev/full",
    "/dev/random",
    "/dev/urandom",
    "/dev/tty",
];

/// Landlock and seccomp, as far as this kernel provides them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Linux {
    landlock: bool,
    seccomp: bool,
}

impl Linux {
    /// The tier this kernel supports, or `None` when it supports neither
    /// mechanism. Probed once per process.
    pub(crate) fn detect() -> Option<Self> {
        static PROBED: OnceLock<Option<Linux>> = OnceLock::new();
        *PROBED.get_or_init(|| {
            let tier = Self {
                landlock: on_own_thread(|| restrict_filesystem(&FsGrant::default())).is_ok(),
                seccomp: on_own_thread(deny_network).is_ok(),
            };
            (tier.landlock || tier.seccomp).then_some(tier)
        })
    }
}

impl Confinement for Linux {
    fn level(&self) -> IsolationLevel {
        IsolationLevel::Os
    }

    fn enforces_network_deny(&self, _grant: &Grant) -> bool {
        self.seccomp
    }

    fn enforces_filesystem(&self, grant: &Grant) -> bool {
        self.landlock && grant.filesystem.is_some()
    }

    fn spawn(&self, cmd: &mut Command, _profile: &Profile, grant: &Grant) -> io::Result<Spawned> {
        let filesystem = grant.filesystem.as_ref().filter(|_| self.landlock);
        let network = !grant.network && self.seccomp;
        if filesystem.is_none() && !network {
            return cmd.spawn().map(Spawned::from);
        }
        let filesystem = filesystem.map(|fs| with_system_paths(fs, cmd));
        on_own_thread(move || {
            if let Some(fs) = &filesystem {
                restrict_filesystem(fs)?;
            }
            if network {
                deny_network()?;
            }
            cmd.spawn().map(Spawned::from)
        })
    }
}

/// Run `f` on a thread of its own, so the restrictions it takes on end with it.
fn on_own_thread<T: Send>(f: impl FnOnce() -> io::Result<T> + Send) -> io::Result<T> {
    std::thread::scope(|scope| {
        scope
            .spawn(f)
            .join()
            .unwrap_or_else(|_| Err(io::Error::other("sandbox thread panicked")))
    })
}

/// `grant` widened by what any program needs to start: the system trees, the
/// devices, the directories on the `PATH` it is given (whatever it finds there
/// it may run), and the program's own installation — its directory and the
/// `lib` and `share` beside it, as named and as resolved.
fn with_system_paths(grant: &FsGrant, cmd: &Command) -> FsGrant {
    let mut fs = grant.clone();
    fs.read.extend(SYSTEM_READ.iter().map(PathBuf::from));
    let path = cmd
        .get_envs()
        .find_map(|(name, value)| (name == "PATH").then_some(value).flatten());
    fs.read.extend(
        path.into_iter()
            .flat_map(std::env::split_paths)
            .filter(|dir| dir.is_absolute()),
    );
    let program = Path::new(cmd.get_program());
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
    fs.write.extend(DEVICES.iter().map(PathBuf::from));
    fs
}

/// Confine the calling thread's filesystem access to `grant`: read (and
/// execute) beneath its read paths, everything beneath its write paths.
fn restrict_filesystem(grant: &FsGrant) -> io::Result<()> {
    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(LANDLOCK_ABI))
        .and_then(Ruleset::create)
        .and_then(|ruleset| {
            ruleset.add_rules(path_beneath_rules(
                &grant.read,
                AccessFs::from_read(LANDLOCK_ABI),
            ))
        })
        .and_then(|ruleset| {
            ruleset.add_rules(path_beneath_rules(
                &grant.write,
                AccessFs::from_all(LANDLOCK_ABI),
            ))
        })
        .and_then(landlock::RulesetCreated::restrict_self)
        .map_err(io::Error::other)?;
    if status.ruleset == RulesetStatus::NotEnforced {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Landlock is not enforced by this kernel",
        ));
    }
    Ok(())
}

/// Refuse, with `EACCES`, every socket in an internet address family on the
/// calling thread, and `io_uring`, which can open one past the filter.
fn deny_network() -> io::Result<()> {
    let arch = TargetArch::try_from(std::env::consts::ARCH).map_err(io::Error::other)?;
    let family = |family: i32| {
        SeccompCondition::new(
            0,
            SeccompCmpArgLen::Dword,
            SeccompCmpOp::Eq,
            u64::from(family.unsigned_abs()),
        )
        .and_then(|condition| SeccompRule::new(vec![condition]))
    };
    let rules = [libc::AF_INET, libc::AF_INET6, libc::AF_PACKET]
        .into_iter()
        .map(family)
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?;
    let filter = SeccompFilter::new(
        [
            (libc::SYS_socket, rules),
            (libc::SYS_io_uring_setup, Vec::new()),
        ]
        .into_iter()
        .collect(),
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EACCES.unsigned_abs()),
        arch,
    )
    .map_err(io::Error::other)?;
    let program: BpfProgram = filter.try_into().map_err(io::Error::other)?;
    seccompiler::apply_filter(&program).map_err(io::Error::other)
}
