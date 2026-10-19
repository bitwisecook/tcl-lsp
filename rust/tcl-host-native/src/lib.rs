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
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tcl_platform::{
    Capabilities, Clock, Env, ExecOutput, Filesystem, Host, HostError, Metadata, Process, StdIo,
    SystemEncoding,
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
    integer_formatter: Option<std::rc::Rc<dyn tcl_platform::NativeIntegerFormatter>>,
}

impl NativeHost {
    /// A host with every native capability.
    #[must_use]
    pub fn new() -> Self {
        let env = NativeEnv::default();
        Self {
            clock: NativeClock,
            stdio: NativeStdIo,
            env: env.clone(),
            fs: NativeFs,
            process: NativeProcess { env },
            allow_filesystem: true,
            allow_process: true,
            integer_formatter: None,
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

    /// Install an explicitly selected, verified native Tcl integer updater.
    /// Ordinary hosts do not load a Tcl library or infer formatter authority.
    #[must_use]
    pub fn with_integer_formatter(
        mut self,
        formatter: tcl_host_c_abi::LoadedNativeIntegerFormatter,
    ) -> Self {
        self.integer_formatter = Some(std::rc::Rc::new(formatter));
        self
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

    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        static ENVIRONMENT: tcl_host_c_abi::NativeNumericEnvironment =
            tcl_host_c_abi::NativeNumericEnvironment;
        tcl_host_c_abi::NativeNumericEnvironment::supported()
            .then_some(&ENVIRONMENT as &dyn tcl_platform::NumericEnvironment)
    }

    fn native_list_sorting_environment(
        &self,
    ) -> Option<&dyn tcl_platform::NativeListSortingEnvironment> {
        static SORTING: tcl_host_c_abi::NativeListSortingEnvironment =
            tcl_host_c_abi::NativeListSortingEnvironment;
        tcl_host_c_abi::NativeListSortingEnvironment::supported()
            .then_some(&SORTING as &dyn tcl_platform::NativeListSortingEnvironment)
    }

    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.integer_formatter.as_deref()
    }

    fn system_encoding(&self) -> SystemEncoding {
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .into_iter()
            .filter_map(|key| self.env.get(key))
            .find(|value| !value.is_empty());
        locale_system_encoding(locale.as_deref())
    }

    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.allow_filesystem.then_some(&self.fs as &dyn Filesystem)
    }

    fn process(&self) -> Option<&dyn Process> {
        self.allow_process.then_some(&self.process as &dyn Process)
    }
}

/// Resolve the locale spellings needed by the runtimes' current encoding
/// surface.  Tcl's Unix bootstrap treats the process `C`/`POSIX` locale as
/// ISO-8859-1; UTF-8 locales retain UTF-8.  Other code pages remain outside
/// the runtimes' deliberately small encoding catalogue and conservatively use
/// UTF-8 rather than inventing a lossy mapping.
fn locale_system_encoding(locale: Option<&str>) -> SystemEncoding {
    let Some(locale) = locale else {
        return SystemEncoding::Iso88591;
    };
    let folded = locale.to_ascii_lowercase();
    if folded == "c" || folded == "posix" {
        return SystemEncoding::Iso88591;
    }
    if folded.contains("utf-8") || folded.contains("utf8") {
        return SystemEncoding::Utf8;
    }
    if folded.contains("iso8859-1") || folded.contains("iso-8859-1") {
        return SystemEncoding::Iso88591;
    }
    SystemEncoding::Utf8
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
/// writes and unsets are kept in a per-host override map (no `unsafe` global
/// mutation), a `None` entry hiding a variable the process has. The map is
/// shared with [`NativeProcess`], so a subprocess sees the same environment.
#[derive(Default, Clone)]
struct NativeEnv {
    overrides: Rc<RefCell<HashMap<String, Option<String>>>>,
}

impl Env for NativeEnv {
    #[cfg(unix)]
    fn native_environment_bytes(&self) -> Option<&dyn tcl_platform::NativeEnvironmentBytes> {
        Some(self)
    }

    fn get(&self, key: &str) -> Option<String> {
        if let Some(v) = self.overrides.borrow().get(key) {
            return v.clone();
        }
        std::env::var(key).ok()
    }

    fn set(&self, key: &str, val: &str) {
        self.overrides
            .borrow_mut()
            .insert(key.to_string(), Some(val.to_string()));
    }

    fn unset(&self, key: &str) {
        self.overrides.borrow_mut().insert(key.to_string(), None);
    }

    fn vars(&self) -> Vec<(String, String)> {
        // `Env` is deliberately a Unicode interface. Unix permits arbitrary
        // bytes in process environment entries, so enumerate with `vars_os`
        // and omit entries that cannot be represented without changing their
        // key or value. `std::env::vars` would panic on the first such entry.
        let mut map: HashMap<String, String> = std::env::vars_os()
            .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
            .collect();
        for (key, value) in self.overrides.borrow().iter() {
            match value {
                Some(value) => map.insert(key.clone(), value.clone()),
                None => map.remove(key),
            };
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

    /// The system user database's `pw_dir` for `user` (`getpwnam_r`).
    fn user_home(&self, user: &str) -> Option<String> {
        user_home(user)
    }
}

#[cfg(unix)]
fn user_home(user: &str) -> Option<String> {
    if user.is_empty() {
        return None;
    }
    let entry = nix::unistd::User::from_name(user).ok()??;
    Some(entry.dir.to_string_lossy().into_owned())
}

#[cfg(not(unix))]
fn user_home(_user: &str) -> Option<String> {
    None
}

#[cfg(unix)]
impl tcl_platform::NativeEnvironmentBytes for NativeEnv {
    fn get_bytes(&self, key: &[u8]) -> Option<Vec<u8>> {
        use std::os::unix::ffi::OsStrExt as _;
        if let Ok(text) = std::str::from_utf8(key)
            && let Some(value) = self.overrides.borrow().get(text)
        {
            return value.as_ref().map(|value| value.as_bytes().to_vec());
        }
        std::env::var_os(std::ffi::OsStr::from_bytes(key))
            .map(|value| value.as_os_str().as_bytes().to_vec())
    }
}

struct NativeSourceFile(std::fs::File);
impl tcl_platform::SourceFile for NativeSourceFile {
    fn read_all(&mut self) -> Result<Vec<u8>, HostError> {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut self.0, &mut bytes).map_err(|error| map_io(&error))?;
        Ok(bytes)
    }
}

struct NativeFs;

impl NativeFs {
    fn copy_path(
        src: &std::path::Path,
        dst: &std::path::Path,
        recursive: bool,
        force: bool,
    ) -> Result<(), HostError> {
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
                Self::copy_path(&entry.path(), &child, true, force)?;
            }
            Ok(())
        } else {
            if !force && dst.exists() {
                return Err(HostError::AlreadyExists);
            }
            std::fs::copy(src, dst).map(|_| ()).map_err(|e| map_io(&e))
        }
    }
}

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

    fn read_bytes(&self, path: &[u8]) -> Result<Vec<u8>, HostError> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt as _;
            std::fs::read(std::ffi::OsStr::from_bytes(path)).map_err(|e| map_io(&e))
        }
        #[cfg(not(unix))]
        {
            let path = std::str::from_utf8(path).map_err(|_| HostError::Unsupported)?;
            self.read(path)
        }
    }

    fn open_source_bytes(
        &self,
        path: &[u8],
    ) -> Result<Box<dyn tcl_platform::SourceFile>, HostError> {
        #[cfg(unix)]
        let file = {
            use std::os::unix::ffi::OsStrExt as _;
            std::fs::File::open(std::ffi::OsStr::from_bytes(path))
        };
        #[cfg(not(unix))]
        let file =
            std::fs::File::open(std::str::from_utf8(path).map_err(|_| HostError::Unsupported)?);
        file.map(|file| Box::new(NativeSourceFile(file)) as Box<dyn tcl_platform::SourceFile>)
            .map_err(|error| map_io(&error))
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
        Self::copy_path(
            std::path::Path::new(source),
            std::path::Path::new(target),
            recursive,
            force,
        )
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

/// Runs subprocesses in the host's environment: the process's own, with the
/// [`NativeEnv`] overrides applied.
struct NativeProcess {
    env: NativeEnv,
}

impl Process for NativeProcess {
    fn run(&self, args: &[&str]) -> Result<ExecOutput, HostError> {
        let (cmd, rest) = args.split_first().ok_or(HostError::NotFound)?;
        let mut command = std::process::Command::new(cmd);
        command.args(rest);
        for (key, value) in self.env.overrides.borrow().iter() {
            // A NUL cannot reach a child's environment: C's `environ` cuts
            // there, and `Command` would refuse to spawn at all.
            if key.contains('\0') {
                continue;
            }
            match value {
                Some(value) => command.env(key, value.split('\0').next().unwrap_or_default()),
                None => command.env_remove(key),
            };
        }
        let output = command.output().map_err(|e| map_io(&e))?;
        Ok(ExecOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn native_environment_bytes_preserve_actual_unix_input_and_current_host_view() {
        use std::os::unix::ffi::OsStringExt as _;

        const MARKER: &str = "NH_NATIVE_ENV_BYTES_CHILD";
        const KEY: &str = "NH_NATIVE_ENV_BYTES_VALUE";
        const EMPTY: &str = "NH_NATIVE_ENV_BYTES_EMPTY";
        const MISSING: &str = "NH_NATIVE_ENV_BYTES_MISSING";
        const RAW_VALUE: &str = "NH_NATIVE_ENV_BYTES_RAW_VALUE";
        let raw_key = b"NH_NATIVE_ENV_BYTES_\xff";
        if std::env::var_os(MARKER).is_some() {
            let host = NativeHost::new();
            let environment = host.env().native_environment_bytes().unwrap();
            assert_eq!(
                environment.get_bytes(KEY.as_bytes()),
                Some(b"actual".to_vec())
            );
            assert_eq!(environment.get_bytes(EMPTY.as_bytes()), Some(Vec::new()));
            assert_eq!(environment.get_bytes(MISSING.as_bytes()), None);
            assert_eq!(environment.get_bytes(raw_key), Some(b"raw-name".to_vec()));
            assert_eq!(
                environment.get_bytes(RAW_VALUE.as_bytes()),
                Some(vec![0xff])
            );
            host.env().set(KEY, "updated");
            assert_eq!(
                environment.get_bytes(KEY.as_bytes()),
                Some(b"updated".to_vec())
            );
            assert_eq!(
                NativeHost::new()
                    .env()
                    .native_environment_bytes()
                    .unwrap()
                    .get_bytes(KEY.as_bytes()),
                Some(b"actual".to_vec())
            );
            assert_eq!(
                std::env::var_os(KEY),
                Some(std::ffi::OsString::from("actual"))
            );
            return;
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("tests::native_environment_bytes_preserve_actual_unix_input_and_current_host_view")
            .arg("--nocapture")
            .env(MARKER, "1")
            .env(KEY, "actual")
            .env(EMPTY, "")
            .env_remove(MISSING)
            .env(std::ffi::OsString::from_vec(raw_key.to_vec()), "raw-name")
            .env(RAW_VALUE, std::ffi::OsString::from_vec(vec![0xff]))
            .status()
            .unwrap();
        assert!(
            status.success(),
            "actual Unix environment child failed: {status}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_file_reads_preserve_distinct_non_unicode_paths() {
        use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("tcl-native-path-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let raw = directory.join(std::ffi::OsString::from_vec(b"package-\xff.tcl".to_vec()));
        let unicode = directory.join("package-\u{fffd}.tcl");
        std::fs::write(&raw, b"RAW").unwrap();
        std::fs::write(&unicode, b"UNICODE").unwrap();
        let host = NativeHost::new();
        let fs = host.filesystem().unwrap();
        assert_eq!(fs.read_bytes(raw.as_os_str().as_bytes()).unwrap(), b"RAW");
        assert_eq!(
            fs.read_bytes(unicode.as_os_str().as_bytes()).unwrap(),
            b"UNICODE"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn recursive_copy_keeps_native_child_paths_and_symlink_targets() {
        use std::os::unix::ffi::OsStringExt as _;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("tcl-native-copy-{}-{stamp}", std::process::id()));
        let source = directory.join("source");
        let target = directory.join("target");
        let raw_name = std::ffi::OsString::from_vec(b"entry-\xff".to_vec());
        let unicode_name = "entry-\u{fffd}";
        std::fs::create_dir_all(source.join(&raw_name)).unwrap();
        std::fs::create_dir_all(source.join(unicode_name)).unwrap();
        std::fs::write(source.join(&raw_name).join("value"), b"RAW").unwrap();
        std::fs::write(source.join(unicode_name).join("value"), b"UNICODE").unwrap();
        let link_name = std::ffi::OsString::from_vec(b"link-\xfe".to_vec());
        let link_target = std::path::PathBuf::from(&raw_name).join("value");
        std::os::unix::fs::symlink(&link_target, source.join(&link_name)).unwrap();
        let host = NativeHost::new();
        host.filesystem()
            .unwrap()
            .copy(
                source.to_str().unwrap(),
                target.to_str().unwrap(),
                true,
                true,
            )
            .unwrap();
        assert_eq!(
            std::fs::read(target.join(&raw_name).join("value")).unwrap(),
            b"RAW"
        );
        assert_eq!(
            std::fs::read(target.join(unicode_name).join("value")).unwrap(),
            b"UNICODE"
        );
        assert_eq!(
            std::fs::read_link(target.join(&link_name)).unwrap(),
            link_target
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn an_unset_hides_the_process_variable_and_a_set_restores_it() {
        let env = NativeEnv::default();
        let path = env.get("PATH");
        env.unset("PATH");
        assert_eq!(env.get("PATH"), None);
        assert!(env.vars().iter().all(|(key, _)| key != "PATH"));
        env.set("PATH", "/x");
        assert_eq!(env.get("PATH").as_deref(), Some("/x"));
        assert!(env.vars().contains(&("PATH".to_owned(), "/x".to_owned())));
        assert!(path.is_some(), "the test process has a PATH");
    }

    /// tclsh hands `set env() val` to a child as the entry `=val`.
    #[cfg(target_os = "linux")]
    #[test]
    fn an_empty_name_reaches_a_subprocess() {
        let host = NativeHost::new();
        host.env().set("", "val");
        let out = host
            .process()
            .expect("a process capability")
            .run(&["cat", "/proc/self/environ"])
            .expect("cat runs");
        assert!(
            out.stdout
                .split(|&byte| byte == 0)
                .any(|entry| entry == b"=val")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_subprocess_runs_in_the_hosts_environment() {
        let host = NativeHost::new();
        host.env().set("TCL_LSP_ENV_PROBE", "seen");
        host.env().unset("HOME");
        host.env().set("TCL_LSP_NUL\0NAME", "x");
        host.env().set("TCL_LSP_NUL_VALUE", "a\0b");
        let out = host
            .process()
            .expect("a process capability")
            .run(&[
                "sh",
                "-c",
                "echo \"$TCL_LSP_ENV_PROBE:${HOME-unset}:$TCL_LSP_NUL_VALUE\"",
            ])
            .expect("sh runs despite NULs in the overrides");
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "seen:unset:a");
    }

    #[cfg(unix)]
    #[test]
    fn a_user_home_comes_from_the_system_user_database() {
        let home = NativeEnv::default()
            .user_home("root")
            .expect("root has a home");
        assert!(home.starts_with('/'), "{home}");
        assert_eq!(NativeEnv::default().user_home("no_such_user_x"), None);
        assert_eq!(NativeEnv::default().user_home(""), None);
        assert_eq!(NativeEnv::default().user_home("a\0b"), None);
    }

    #[test]
    fn system_encoding_follows_tcl_unix_locale_defaults() {
        assert_eq!(locale_system_encoding(None), SystemEncoding::Iso88591);
        assert_eq!(locale_system_encoding(Some("C")), SystemEncoding::Iso88591);
        assert_eq!(
            locale_system_encoding(Some("POSIX")),
            SystemEncoding::Iso88591
        );
        assert_eq!(
            locale_system_encoding(Some("C.UTF-8")),
            SystemEncoding::Utf8
        );
        assert_eq!(
            locale_system_encoding(Some("en_GB.utf8")),
            SystemEncoding::Utf8
        );
        assert_eq!(
            locale_system_encoding(Some("en_US.ISO-8859-1")),
            SystemEncoding::Iso88591
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_environment_skips_non_unicode_entries_without_panicking() {
        use std::os::unix::ffi::OsStringExt;

        const CHILD_MARKER: &str = "TCL_HOST_NATIVE_NON_UNICODE_CHILD";
        const UNICODE_SENTINEL: &str = "TCL_HOST_NATIVE_UNICODE_SENTINEL";
        if std::env::var_os(CHILD_MARKER).is_some() {
            let vars = NativeHost::new().env().vars();
            assert!(
                vars.iter()
                    .any(|(key, value)| { key == UNICODE_SENTINEL && value == "visible" })
            );
            return;
        }

        let executable = std::env::current_exe().expect("current test executable");
        let invalid_name = std::ffi::OsString::from_vec(b"TCL_HOST_NATIVE_\xff".to_vec());
        let invalid_value = std::ffi::OsString::from_vec(vec![0xff]);
        let status = std::process::Command::new(executable)
            .arg("--exact")
            .arg("tests::native_environment_skips_non_unicode_entries_without_panicking")
            .arg("--nocapture")
            .env(CHILD_MARKER, "1")
            .env(UNICODE_SENTINEL, "visible")
            .env(invalid_name, "ignored")
            .env("TCL_HOST_NATIVE_INVALID_VALUE", invalid_value)
            .status()
            .expect("run child test process");
        assert!(status.success(), "child test process failed: {status}");
    }
}
