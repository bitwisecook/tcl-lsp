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

//! `NativeHost` — the std-backed [`Host`] for the native `TclVM` target.
//!
//! This is the per-target half of the capability seam: the portable command
//! bodies in `tcl-cmd-core::platform` call through the [`tcl_platform`]
//! capability traits, and `NativeHost` services them with real `std` syscalls.
//! It deliberately has all capabilities (filesystem, subprocess, …), in contrast
//! to the WASM runtime's host, which returns `None` for what WASI/the browser
//! cannot do.
//!
//! [`NativeHost::sandboxed`] flips the conditional capabilities off — the same
//! mechanism a restricted (WASM) host uses — so the "platform cannot do this"
//! path is exercisable on native too.
//!
//! `Env::set` keeps an internal override map rather than mutating the global
//! process environment: `std::env::set_var` is `unsafe` under edition 2024
//! (thread-unsafe), and this crate is `forbid(unsafe)`. A per-interp env view is
//! the correct model anyway.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tcl_platform::{
    Capabilities, Clock, Env, ExecOutput, Filesystem, Host, HostError, Metadata, Process, StdIo,
};

/// The native, std-backed host. Holds its capability objects so the `Host`
/// accessors can hand out `&dyn` references.
pub struct NativeHost {
    clock: NativeClock,
    stdio: NativeStdIo,
    env: NativeEnv,
    fs: NativeFs,
    process: NativeProcess,
    allow_filesystem: bool,
    allow_process: bool,
}

impl NativeHost {
    /// A host with every native capability.
    #[must_use]
    pub fn new() -> Self {
        Self {
            clock: NativeClock,
            stdio: NativeStdIo,
            env: NativeEnv::default(),
            fs: NativeFs,
            process: NativeProcess,
            allow_filesystem: true,
            allow_process: true,
        }
    }

    /// A host with the *conditional* capabilities (subprocess) switched off —
    /// the restricted posture a WASM host has, exercisable natively.
    #[must_use]
    pub fn sandboxed() -> Self {
        Self {
            allow_process: false,
            ..Self::new()
        }
    }
}

impl Default for NativeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Host for NativeHost {
    fn capabilities(&self) -> Capabilities {
        let mut caps = Capabilities::empty();
        if self.allow_filesystem {
            caps = caps.union(Capabilities::FILESYSTEM);
        }
        if self.allow_process {
            caps = caps.union(Capabilities::PROCESS);
        }
        caps
    }

    fn clock(&self) -> &dyn Clock {
        &self.clock
    }

    fn stdio(&self) -> &dyn StdIo {
        &self.stdio
    }

    fn env(&self) -> &dyn Env {
        &self.env
    }

    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.allow_filesystem.then_some(&self.fs as &dyn Filesystem)
    }

    fn process(&self) -> Option<&dyn Process> {
        self.allow_process.then_some(&self.process as &dyn Process)
    }
}

/// Map a `std::io::Error` onto the host-neutral [`HostError`].
fn map_io(e: &std::io::Error) -> HostError {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => HostError::NotFound,
        ErrorKind::PermissionDenied => HostError::Permission,
        ErrorKind::AlreadyExists => HostError::AlreadyExists,
        ErrorKind::WouldBlock => HostError::WouldBlock,
        _ => HostError::Io(e.to_string()),
    }
}

struct NativeClock;

impl Clock for NativeClock {
    fn now_secs(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_secs()).ok())
            .unwrap_or(0)
    }

    fn now_millis(&self) -> i128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| i128::try_from(d.as_millis()).ok())
            .unwrap_or(0)
    }

    fn now_micros(&self) -> i128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| i128::try_from(d.as_micros()).ok())
            .unwrap_or(0)
    }
    // `local_offset_secs` keeps the trait default (0 = UTC): the std host has no
    // timezone database, so local time equals UTC until a TZ-capable host wires it.
}

struct NativeStdIo;

impl StdIo for NativeStdIo {
    fn write_stdout(&self, bytes: &[u8]) {
        let _ = std::io::stdout().write_all(bytes);
    }

    fn write_stderr(&self, bytes: &[u8]) {
        let _ = std::io::stderr().write_all(bytes);
    }

    fn flush_stdout(&self) {
        let _ = std::io::stdout().flush();
    }

    fn flush_stderr(&self) {
        let _ = std::io::stderr().flush();
    }
}

/// Environment view: reads fall through to the real process environment, but
/// writes are kept in a per-host override map (no `unsafe` global mutation).
#[derive(Default)]
struct NativeEnv {
    overrides: RefCell<HashMap<String, String>>,
}

impl Env for NativeEnv {
    fn get(&self, key: &str) -> Option<String> {
        if let Some(v) = self.overrides.borrow().get(key) {
            return Some(v.clone());
        }
        std::env::var(key).ok()
    }

    fn set(&self, key: &str, val: &str) {
        self.overrides
            .borrow_mut()
            .insert(key.to_string(), val.to_string());
    }

    fn vars(&self) -> Vec<(String, String)> {
        let mut map: HashMap<String, String> = std::env::vars().collect();
        for (k, v) in self.overrides.borrow().iter() {
            map.insert(k.clone(), v.clone());
        }
        map.into_iter().collect()
    }

    fn cwd(&self) -> Result<String, HostError> {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| map_io(&e))
    }

    fn chdir(&self, path: &str) -> Result<(), HostError> {
        std::env::set_current_dir(path).map_err(|e| map_io(&e))
    }

    fn current_exe(&self) -> Option<String> {
        std::env::current_exe()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    }
}

struct NativeFs;

impl Filesystem for NativeFs {
    fn exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    fn metadata(&self, path: &str) -> Result<Metadata, HostError> {
        let m = std::fs::metadata(path).map_err(|e| map_io(&e))?;
        Ok(meta_from(&m))
    }

    fn symlink_metadata(&self, path: &str) -> Result<Metadata, HostError> {
        let m = std::fs::symlink_metadata(path).map_err(|e| map_io(&e))?;
        Ok(meta_from(&m))
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, HostError> {
        std::fs::read(path).map_err(|e| map_io(&e))
    }

    fn write(&self, path: &str, data: &[u8]) -> Result<(), HostError> {
        std::fs::write(path, data).map_err(|e| map_io(&e))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, HostError> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| map_io(&e))? {
            let entry = entry.map_err(|e| map_io(&e))?;
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        Ok(names)
    }

    fn create_dir_all(&self, path: &str) -> Result<(), HostError> {
        std::fs::create_dir_all(path).map_err(|e| map_io(&e))
    }

    fn create_dir(&self, path: &str) -> Result<(), HostError> {
        std::fs::create_dir(path).map_err(|e| map_io(&e))
    }

    fn remove(&self, path: &str, recursive: bool) -> Result<(), HostError> {
        let meta = std::fs::symlink_metadata(path).map_err(|e| map_io(&e))?;
        if meta.is_dir() {
            if recursive {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_dir(path)
            }
        } else {
            std::fs::remove_file(path)
        }
        .map_err(|e| map_io(&e))
    }

    fn rename(&self, source: &str, target: &str, force: bool) -> Result<(), HostError> {
        if force {
            let _ = std::fs::remove_file(target);
            let _ = std::fs::remove_dir_all(target);
        }
        std::fs::rename(source, target).map_err(|e| map_io(&e))
    }

    fn copy(
        &self,
        source: &str,
        target: &str,
        recursive: bool,
        force: bool,
    ) -> Result<(), HostError> {
        let src = std::path::Path::new(source);
        let dst = std::path::Path::new(target);
        let meta = std::fs::symlink_metadata(src).map_err(|e| map_io(&e))?;
        if meta.file_type().is_symlink() {
            let link = std::fs::read_link(src).map_err(|e| map_io(&e))?;
            if force {
                let _ = std::fs::remove_file(dst);
            }
            create_link(dst, &link, false)
        } else if meta.is_dir() {
            if !recursive {
                return Err(HostError::Io("is a directory".to_string()));
            }
            std::fs::create_dir_all(dst).map_err(|e| map_io(&e))?;
            for entry in std::fs::read_dir(src).map_err(|e| map_io(&e))? {
                let entry = entry.map_err(|e| map_io(&e))?;
                let child = dst.join(entry.file_name());
                self.copy(
                    &entry.path().to_string_lossy(),
                    &child.to_string_lossy(),
                    true,
                    force,
                )?;
            }
            Ok(())
        } else {
            if !force && dst.exists() {
                return Err(HostError::AlreadyExists);
            }
            std::fs::copy(src, dst).map(|_| ()).map_err(|e| map_io(&e))
        }
    }

    fn readlink(&self, path: &str) -> Result<String, HostError> {
        std::fs::read_link(path)
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| map_io(&e))
    }

    fn link(&self, link: &str, target: &str, hard: bool) -> Result<(), HostError> {
        create_link(
            std::path::Path::new(link),
            std::path::Path::new(target),
            hard,
        )
    }

    fn set_times(
        &self,
        path: &str,
        atime_secs: Option<i64>,
        mtime_secs: Option<i64>,
    ) -> Result<(), HostError> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .open(path)
            .map_err(|e| map_io(&e))?;
        let mut times = std::fs::FileTimes::new();
        if let Some(secs) = atime_secs {
            times = times.set_accessed(epoch_time(secs));
        }
        if let Some(secs) = mtime_secs {
            times = times.set_modified(epoch_time(secs));
        }
        file.set_times(times).map_err(|e| map_io(&e))
    }
}

fn epoch_time(secs: i64) -> SystemTime {
    if secs < 0 {
        UNIX_EPOCH - Duration::from_secs(secs.unsigned_abs())
    } else {
        UNIX_EPOCH + Duration::from_secs(secs.cast_unsigned())
    }
}

fn create_link(
    link: &std::path::Path,
    target: &std::path::Path,
    hard: bool,
) -> Result<(), HostError> {
    if hard {
        std::fs::hard_link(target, link).map_err(|e| map_io(&e))
    } else {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).map_err(|e| map_io(&e))
        }
        #[cfg(windows)]
        {
            let is_dir = std::fs::metadata(target)
                .map(|m| m.is_dir())
                .unwrap_or(false);
            if is_dir {
                std::os::windows::fs::symlink_dir(target, link).map_err(|e| map_io(&e))
            } else {
                std::os::windows::fs::symlink_file(target, link).map_err(|e| map_io(&e))
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (link, target);
            Err(HostError::Unsupported)
        }
    }
}

/// Project a `std::fs::Metadata` onto the portable [`Metadata`]. `is_symlink`
/// comes from the file type, so it is `true` only when the source was a
/// non-following `symlink_metadata` of an actual link (a following `metadata`
/// resolves the link and reports the target).
fn meta_from(m: &std::fs::Metadata) -> Metadata {
    let ft = m.file_type();
    let mtime_secs = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0);
    // Executability from the Unix mode bits; elsewhere (no Unix perms) we cannot
    // tell, so assume yes rather than hiding a runnable file.
    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        m.permissions().mode() & 0o111 != 0
    };
    #[cfg(not(unix))]
    let executable = true;
    #[cfg(unix)]
    let (dev, ino, nlink, uid, gid, mode, blocks, blksize, atime_secs, ctime_secs) = {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        (
            m.dev(),
            m.ino(),
            m.nlink(),
            u64::from(m.uid()),
            u64::from(m.gid()),
            m.permissions().mode(),
            m.blocks(),
            m.blksize(),
            m.atime(),
            m.ctime(),
        )
    };
    #[cfg(not(unix))]
    let (dev, ino, nlink, uid, gid, mode, blocks, blksize, atime_secs, ctime_secs) =
        (0, 0, 1, 0, 0, 0, 0, 0, 0, 0);
    Metadata {
        is_dir: ft.is_dir(),
        is_file: ft.is_file(),
        is_symlink: ft.is_symlink(),
        executable,
        len: m.len(),
        mtime_secs,
        dev,
        ino,
        nlink,
        uid,
        gid,
        mode,
        blocks,
        blksize,
        atime_secs,
        ctime_secs,
    }
}

struct NativeProcess;

impl Process for NativeProcess {
    fn run(&self, args: &[&str]) -> Result<ExecOutput, HostError> {
        let (cmd, rest) = args.split_first().ok_or(HostError::NotFound)?;
        let output = std::process::Command::new(cmd)
            .args(rest)
            .output()
            .map_err(|e| map_io(&e))?;
        Ok(ExecOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}
