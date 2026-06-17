//! The platform (host-capability) seam.
//!
//! The clean boundary between *what a Tcl command computes* (portable, in
//! `tcl-cmd-core`) and *what the host environment can do* (per-target). This
//! crate is pure trait + type definitions — no syscalls, no `std::fs`/`net`/
//! `process` — so it is a dependency-free, `wasm32`-clean leaf.
//!
//! # The capability model
//!
//! The native TclVM/runtime can do things WASM+WASI cannot — subprocess,
//! sockets, threads, full `stat`. Rather than force every host to implement
//! everything, a [`Host`] exposes:
//!
//! - **mandatory** facilities ([`Clock`], [`StdIo`], [`Env`]) as `&dyn`, and
//! - **conditional** facilities ([`Filesystem`], [`Sockets`], [`Process`]) as
//!   `Option<&dyn>`.
//!
//! A platform that lacks a facility returns `None`; the shared command body in
//! `tcl-cmd-core` then produces the faithful Tcl error (`HostError::Unsupported`
//! → e.g. `exec` "not supported"). The trait surface is uniform across builds;
//! the impls are `#[cfg]`-selected per target, so a browser build need carry no
//! process-spawn code at all.

/// What a host environment can do. A uniform query over the [`Host`] regardless
/// of build; the conditional accessors ([`Host::filesystem`] etc.) still gate
/// actual use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities(u32);

impl Capabilities {
    /// Whole-file and directory access ([`Filesystem`]).
    pub const FILESYSTEM: Self = Self(1 << 0);
    /// Stream sockets ([`Sockets`]).
    pub const SOCKETS: Self = Self(1 << 1);
    /// Subprocess execution ([`Process`]).
    pub const PROCESS: Self = Self(1 << 2);
    /// OS threads.
    pub const THREADS: Self = Self(1 << 3);

    /// The empty capability set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Whether every capability in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// The union of two capability sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for Capabilities {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// A host operation failure. Maps to the faithful Tcl error in `tcl-cmd-core`;
/// [`HostError::reason`] renders the POSIX-style reason clause Tcl appends after
/// `couldn't open "…": `.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    /// The host does not provide this facility at all (e.g. `exec` under WASI).
    Unsupported,
    /// No such file or directory.
    NotFound,
    /// Permission denied.
    Permission,
    /// The target already exists.
    AlreadyExists,
    /// A non-blocking operation would block.
    WouldBlock,
    /// Any other I/O error, with a rendered reason string.
    Io(String),
}

impl HostError {
    /// The POSIX-style reason clause (the text Tcl puts after the `: `), e.g.
    /// `no such file or directory`.
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            HostError::Unsupported => "operation not supported".to_string(),
            HostError::NotFound => "no such file or directory".to_string(),
            HostError::Permission => "permission denied".to_string(),
            HostError::AlreadyExists => "file already exists".to_string(),
            HostError::WouldBlock => "operation would block".to_string(),
            HostError::Io(msg) => msg.clone(),
        }
    }
}

impl core::fmt::Display for HostError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.reason())
    }
}

impl std::error::Error for HostError {}

/// Metadata about a filesystem entry (`file stat` / `file exists` / `file type`
/// backing).
// A handful of independent kind/permission flags mirroring `stat`; an enum would
// fight the fact that they are orthogonal queries (`file type` vs `-types x`).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// Whether the entry is a regular file.
    pub is_file: bool,
    /// Whether the entry is itself a symbolic link. Only ever `true` from
    /// [`Filesystem::symlink_metadata`] (the non-following stat); the following
    /// [`Filesystem::metadata`] resolves the link, so it reports the target's
    /// kind with `is_symlink == false`.
    pub is_symlink: bool,
    /// Whether the entry is executable (`file executable`, `glob -types x`).
    /// Best-effort: a native host reads the Unix execute bits; a restricted host
    /// (WASI, browser) that cannot tell may report `false` or `true` uniformly.
    pub executable: bool,
    /// Length in bytes.
    pub len: u64,
    /// Last-modified time, seconds since the Unix epoch.
    pub mtime_secs: i64,
}

/// The output of a finished subprocess ([`Process::run`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOutput {
    /// The process exit status.
    pub status: i32,
    /// Captured standard output.
    pub stdout: Vec<u8>,
    /// Captured standard error.
    pub stderr: Vec<u8>,
}

/// Whole-file and directory access. Streaming channels (the per-fd table,
/// buffering, encoding) layer on top in a later milestone; this is the
/// stateless surface the Phase-1 path/`glob`/`file` commands need.
pub trait Filesystem {
    /// Whether `path` exists.
    fn exists(&self, path: &str) -> bool;
    /// Metadata for `path`, following symlinks (so a link reports its target's
    /// kind). Backs `file stat`/`size`/`isdirectory`/`isfile`.
    fn metadata(&self, path: &str) -> Result<Metadata, HostError>;
    /// Metadata for `path` *without* following a final symlink (so a link
    /// reports `is_symlink`, not its target). Backs `file type`/`lstat` and
    /// `glob -types l`. Defaults to [`metadata`](Filesystem::metadata) for a
    /// host with no symlink notion (e.g. a flat in-memory VFS).
    fn symlink_metadata(&self, path: &str) -> Result<Metadata, HostError> {
        self.metadata(path)
    }
    /// Read an entire file.
    fn read(&self, path: &str) -> Result<Vec<u8>, HostError>;
    /// Write (creating/truncating) an entire file.
    fn write(&self, path: &str, data: &[u8]) -> Result<(), HostError>;
    /// The entry names (not full paths) directly under `path`.
    fn read_dir(&self, path: &str) -> Result<Vec<String>, HostError>;
    /// Create `path` and any missing parents.
    fn create_dir_all(&self, path: &str) -> Result<(), HostError>;
    /// Remove `path` (a file, or a directory when `recursive`).
    fn remove(&self, path: &str, recursive: bool) -> Result<(), HostError>;
}

/// The wall and monotonic clock (`clock seconds`/`milliseconds`). Mandatory —
/// every host can report time (the browser via a host import).
pub trait Clock {
    /// Seconds since the Unix epoch.
    fn now_secs(&self) -> i64;
    /// Milliseconds since the Unix epoch.
    fn now_millis(&self) -> i128;
}

/// Standard output/error sinks (`puts`). Mandatory — the browser routes these to
/// a host console import.
pub trait StdIo {
    /// Write bytes to standard output.
    fn write_stdout(&self, bytes: &[u8]);
    /// Write bytes to standard error.
    fn write_stderr(&self, bytes: &[u8]);
    /// Flush buffered standard output (`flush stdout`). Defaults to a no-op for
    /// a host that writes synchronously (the browser console).
    fn flush_stdout(&self) {}
    /// Flush buffered standard error (`flush stderr`). Defaults to a no-op.
    fn flush_stderr(&self) {}
}

/// Environment variables and the working directory (`env`, `pwd`, `cd`).
/// Mandatory — virtualised on the browser.
pub trait Env {
    /// The value of environment variable `key`.
    fn get(&self, key: &str) -> Option<String>;
    /// Set environment variable `key` to `val`.
    fn set(&self, key: &str, val: &str);
    /// All environment variables.
    fn vars(&self) -> Vec<(String, String)>;
    /// The current working directory (`pwd`).
    fn cwd(&self) -> Result<String, HostError>;
    /// Change the working directory (`cd`).
    fn chdir(&self, path: &str) -> Result<(), HostError>;
    /// The path of the running executable (`info nameofexecutable`,
    /// `Tcl_GetNameOfExecutable`). `None` on a host with no such notion (the
    /// browser, and WASI where there is no host-process path).
    fn current_exe(&self) -> Option<String> {
        None
    }
}

/// Stream sockets (`socket`). Conditional — absent under WASI preview 1 and in
/// the browser (where only a host-mediated `WebSocket` is available). The method
/// surface lands with the channel layer; the marker trait reserves the seam.
pub trait Sockets {}

/// Subprocess execution (`exec`, `open |pipe`). Conditional — absent on every
/// WASM target. A host without it makes [`Host::process`] return `None`.
pub trait Process {
    /// Run `args[0]` with `args[1..]`, capturing output to completion.
    fn run(&self, args: &[&str]) -> Result<ExecOutput, HostError>;
}

/// The host environment a Tcl runtime executes in: the aggregate of the
/// capabilities above.
///
/// Mandatory facilities are always present (`&dyn`); conditional ones are
/// `Option<&dyn>` so a restricted host (WASI, browser) reports absence rather
/// than panicking. [`Host::capabilities`] is the uniform up-front query.
pub trait Host {
    /// The capability set this host provides.
    fn capabilities(&self) -> Capabilities;

    /// The clock (always present).
    fn clock(&self) -> &dyn Clock;
    /// Standard output/error (always present).
    fn stdio(&self) -> &dyn StdIo;
    /// Environment + working directory (always present).
    fn env(&self) -> &dyn Env;

    /// The filesystem, or `None` on a host without one (e.g. a no-VFS browser).
    fn filesystem(&self) -> Option<&dyn Filesystem> {
        None
    }
    /// Stream sockets, or `None` (WASI p1, browser).
    fn sockets(&self) -> Option<&dyn Sockets> {
        None
    }
    /// Subprocess execution, or `None` (every WASM target).
    fn process(&self) -> Option<&dyn Process> {
        None
    }
}
