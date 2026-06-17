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
use std::time::{SystemTime, UNIX_EPOCH};

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
}

struct NativeStdIo;

impl StdIo for NativeStdIo {
    fn write_stdout(&self, bytes: &[u8]) {
        let _ = std::io::stdout().write_all(bytes);
    }

    fn write_stderr(&self, bytes: &[u8]) {
        let _ = std::io::stderr().write_all(bytes);
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
    Metadata {
        is_dir: ft.is_dir(),
        is_file: ft.is_file(),
        is_symlink: ft.is_symlink(),
        executable,
        len: m.len(),
        mtime_secs,
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
