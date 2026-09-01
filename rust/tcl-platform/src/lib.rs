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
    /// POSIX stat identity and mode fields when the host exposes them.
    pub dev: u64,
    pub ino: u64,
    pub nlink: u64,
    pub uid: u64,
    pub gid: u64,
    pub mode: u32,
    pub blocks: u64,
    pub blksize: u64,
    pub atime_secs: i64,
    pub ctime_secs: i64,
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
/// buffering, encoding) layer on top separately; this is the stateless
/// surface the path/`glob`/`file` commands need.
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
    /// Create exactly one directory. Unlike [`Self::create_dir_all`], a
    /// missing parent is an error; this is the primitive Tcl's `file tempdir`
    /// needs when a template supplies its containing directory.
    fn create_dir(&self, path: &str) -> Result<(), HostError> {
        self.create_dir_all(path)
    }
    /// Remove `path` (a file, or a directory when `recursive`).
    fn remove(&self, path: &str, recursive: bool) -> Result<(), HostError>;
    /// Rename or move an entry without following a final symlink.
    fn rename(&self, _source: &str, _target: &str, _force: bool) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
    /// Copy a file, directory, or symlink. The `recursive` flag permits a
    /// directory tree; `force` permits replacement where the host supports it.
    fn copy(
        &self,
        _source: &str,
        _target: &str,
        _recursive: bool,
        _force: bool,
    ) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
    /// Read the target of a symbolic link without following it.
    fn readlink(&self, _path: &str) -> Result<String, HostError> {
        Err(HostError::Unsupported)
    }
    /// Create a symbolic or hard link. Hosts without link support report
    /// `Unsupported` rather than emulating links by copying bytes.
    fn link(&self, _link: &str, _target: &str, _hard: bool) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
    /// Update access and modification times, in Unix epoch seconds.
    fn set_times(
        &self,
        _path: &str,
        _atime_secs: Option<i64>,
        _mtime_secs: Option<i64>,
    ) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
}

/// The wall and monotonic clock (`clock seconds`/`milliseconds`). Mandatory —
/// every host can report time (the browser via a host import).
pub trait Clock {
    /// Seconds since the Unix epoch.
    fn now_secs(&self) -> i64;
    /// Milliseconds since the Unix epoch.
    fn now_millis(&self) -> i128;
    /// Microseconds since the Unix epoch (`clock microseconds`/`clock clicks`).
    /// Defaults to millisecond precision; a host with finer resolution overrides.
    fn now_micros(&self) -> i128 {
        self.now_millis() * 1000
    }
    /// The local timezone's offset from UTC in **seconds east of UTC**, at the
    /// instant `at_secs` (so DST is accounted for). Backs `clock format`/`scan`
    /// without an explicit `-gmt`. A host with no timezone database (the std host
    /// today, and a bare browser) returns `0` — local time then equals UTC.
    fn local_offset_secs(&self, _at_secs: i64) -> i32 {
        0
    }
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

/// Canonical bootstrap schema for the predefined `tcl_platform` array.
///
/// The two interpreters supply the few host- and engine-dependent values via
/// [`Values`], then install every [`entries`] row.  Keeping the key set, its
/// portable defaults, and the safe-interpreter scrub policy in this leaf crate
/// prevents a runtime from silently acquiring a different platform surface.
pub mod bootstrap {
    use super::backend;

    /// Shared-library suffix exposed by `info sharedlibextension` for the
    /// canonical Unix platform both Rust interpreters currently publish.
    pub const SHARED_LIBRARY_EXTENSION: &str = ".so";

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ValueSource {
        Literal(&'static str),
        OsVersion,
        Machine,
        User,
        Runtime,
        RuntimeVersion,
        Wasm,
        Wasi,
        WasiVersion,
        Ebpf,
    }

    /// Engine-provided values for the non-constant platform facts.
    ///
    /// Providers intentionally remain strings: Tcl exposes every
    /// `tcl_platform` element as a string and a restricted host may only know
    /// the empty value for a fact such as the operating-system release.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Values<'a> {
        pub os_version: &'a str,
        pub machine: &'a str,
        pub user: &'a str,
        pub runtime: &'a str,
        pub runtime_version: &'a str,
        pub wasm: &'a str,
        pub wasi: &'a str,
        pub wasi_version: &'a str,
        pub ebpf: &'a str,
    }

    /// One canonical `tcl_platform` element and its safe-interpreter policy.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Entry {
        name: &'static str,
        source: ValueSource,
        scrub_in_safe: bool,
    }

    impl Entry {
        const fn new(name: &'static str, source: ValueSource, scrub_in_safe: bool) -> Self {
            Self {
                name,
                source,
                scrub_in_safe,
            }
        }

        /// The array-element name.
        #[must_use]
        pub const fn name(self) -> &'static str {
            self.name
        }

        /// Resolve this element for an interpreter's supplied values.
        #[must_use]
        pub fn value<'a>(self, values: &'a Values<'a>) -> &'a str {
            match self.source {
                ValueSource::Literal(value) => value,
                ValueSource::OsVersion => values.os_version,
                ValueSource::Machine => values.machine,
                ValueSource::User => values.user,
                ValueSource::Runtime => values.runtime,
                ValueSource::RuntimeVersion => values.runtime_version,
                ValueSource::Wasm => values.wasm,
                ValueSource::Wasi => values.wasi,
                ValueSource::WasiVersion => values.wasi_version,
                ValueSource::Ebpf => values.ebpf,
            }
        }

        /// Whether `Tcl_MakeSafe` must remove this host-revealing element.
        #[must_use]
        pub const fn scrub_in_safe(self) -> bool {
            self.scrub_in_safe
        }
    }

    const ENTRIES: &[Entry] = &[
        Entry::new("platform", ValueSource::Literal("unix"), false),
        Entry::new("os", ValueSource::Literal("Linux"), true),
        Entry::new("osVersion", ValueSource::OsVersion, true),
        Entry::new("machine", ValueSource::Machine, true),
        Entry::new("byteOrder", ValueSource::Literal("littleEndian"), false),
        Entry::new("wordSize", ValueSource::Literal("8"), false),
        Entry::new("pointerSize", ValueSource::Literal("8"), false),
        Entry::new("pathSeparator", ValueSource::Literal(":"), false),
        Entry::new("engine", ValueSource::Literal("Tcl"), false),
        // Tcl itself keeps `threaded` in a safe child.  It reports build
        // capability rather than host identity; an embedder may change it to
        // `1` after bootstrap when it installs thread support.
        Entry::new("threaded", ValueSource::Literal("0"), false),
        Entry::new("user", ValueSource::User, true),
        Entry::new(backend::key::RUNTIME, ValueSource::Runtime, true),
        Entry::new(
            backend::key::RUNTIME_VERSION,
            ValueSource::RuntimeVersion,
            true,
        ),
        Entry::new(backend::key::WASM, ValueSource::Wasm, true),
        Entry::new(backend::key::WASI, ValueSource::Wasi, true),
        Entry::new(backend::key::WASI_VERSION, ValueSource::WasiVersion, true),
        Entry::new(backend::key::EBPF, ValueSource::Ebpf, true),
    ];

    /// The complete platform array schema, in deterministic installation order.
    #[must_use]
    pub fn entries() -> &'static [Entry] {
        ENTRIES
    }

    /// Keys removed when an interpreter becomes safe, derived from [`entries`].
    pub fn safe_scrub_keys() -> impl Iterator<Item = &'static str> {
        ENTRIES
            .iter()
            .copied()
            .filter(|entry| entry.scrub_in_safe())
            .map(Entry::name)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const VALUES: Values<'static> = Values {
            os_version: "kernel",
            machine: "machine",
            user: "user",
            runtime: "runtime",
            runtime_version: "runtime-version",
            wasm: "wasm",
            wasi: "wasi",
            wasi_version: "wasi-version",
            ebpf: "ebpf",
        };

        #[test]
        fn schema_has_unique_expected_keys_and_provider_values() {
            let names = entries()
                .iter()
                .map(|entry| entry.name())
                .collect::<Vec<_>>();
            assert_eq!(
                names,
                [
                    "platform",
                    "os",
                    "osVersion",
                    "machine",
                    "byteOrder",
                    "wordSize",
                    "pointerSize",
                    "pathSeparator",
                    "engine",
                    "threaded",
                    "user",
                    "runtime",
                    "runtimeVersion",
                    "wasm",
                    "wasi",
                    "wasiVersion",
                    "ebpf",
                ]
            );
            let mut unique = names.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), names.len());

            for (name, expected) in [
                ("osVersion", VALUES.os_version),
                ("machine", VALUES.machine),
                ("user", VALUES.user),
                ("runtime", VALUES.runtime),
                ("runtimeVersion", VALUES.runtime_version),
                ("wasm", VALUES.wasm),
                ("wasi", VALUES.wasi),
                ("wasiVersion", VALUES.wasi_version),
                ("ebpf", VALUES.ebpf),
            ] {
                let entry = entries()
                    .iter()
                    .find(|entry| entry.name() == name)
                    .expect("provider-backed platform key");
                assert_eq!(entry.value(&VALUES), expected);
            }
        }

        #[test]
        fn safe_scrub_is_derived_from_the_schema() {
            let scrubbed = safe_scrub_keys().collect::<Vec<_>>();
            assert_eq!(
                scrubbed,
                [
                    "os",
                    "osVersion",
                    "machine",
                    "user",
                    "runtime",
                    "runtimeVersion",
                    "wasm",
                    "wasi",
                    "wasiVersion",
                    "ebpf",
                ]
            );
            for portable in [
                "platform",
                "byteOrder",
                "wordSize",
                "pointerSize",
                "pathSeparator",
                "engine",
                "threaded",
            ] {
                assert!(!scrubbed.contains(&portable));
            }
        }
    }
}

/// Backend introspection for the `tcl_platform` keys the test-suite
/// backend-constraint overlay reads to decide which upstream tcltest tests a
/// given build can run.
///
/// The compiled-in values below are detected from the build's `cfg`, so they
/// describe *this* binary truthfully. A runtime publishes them under the
/// canonical [`Key`] names after the standard `tcl_platform` fields; it may
/// override any one from its environment seam first (e.g. `TCL_EBPF_SPEC`), so
/// a native binary can be asked to evaluate the skip lists as if it were a
/// different backend — the only way to reason about the eBPF target, which
/// cannot host a full interpreter at all.
///
/// Schema (all string-valued; an empty string means "not this target"):
///
/// | key              | meaning                                              |
/// |------------------|------------------------------------------------------|
/// | `runtime`        | interpreter implementation: `bytecode`/`treewalk`/`ebpf` |
/// | `runtimeVersion` | that implementation's host (crate) version           |
/// | `wasm`           | wasm spec version when a wasm build, else empty      |
/// | `wasi`           | WASI spec version when a WASI build, else empty      |
/// | `wasiVersion`    | WASI host/preview identifier, else empty            |
/// | `ebpf`           | eBPF target version when an eBPF build, else empty  |
pub mod backend {
    /// Canonical `tcl_platform` key names, so every runtime publishes one schema.
    pub mod key {
        /// Interpreter implementation kind (`bytecode`/`treewalk`/`ebpf`).
        pub const RUNTIME: &str = "runtime";
        /// The implementation's host (crate) version.
        pub const RUNTIME_VERSION: &str = "runtimeVersion";
        /// wasm spec version (empty when not a wasm build).
        pub const WASM: &str = "wasm";
        /// WASI spec version (empty when not a WASI build).
        pub const WASI: &str = "wasi";
        /// WASI host/preview identifier (empty when not a WASI build).
        pub const WASI_VERSION: &str = "wasiVersion";
        /// eBPF target version (empty when not an eBPF build).
        pub const EBPF: &str = "ebpf";
    }

    /// The wasm spec version this build targets, or `""` when not a wasm build.
    /// The runtime currently targets the wasm 2.0 feature set under WASI.
    #[must_use]
    pub const fn compiled_wasm_spec() -> &'static str {
        if cfg!(target_arch = "wasm32") {
            "2.0"
        } else {
            ""
        }
    }

    /// The WASI spec version this build targets, or `""` when not a WASI build.
    /// `preview1` (wasm32-wasip1) vs `0.2` (wasm32-wasip2) is read from the
    /// target environment.
    #[must_use]
    pub const fn compiled_wasi_spec() -> &'static str {
        if cfg!(all(target_arch = "wasm32", target_os = "wasi")) {
            if cfg!(target_env = "p2") {
                "0.2"
            } else {
                "preview1"
            }
        } else {
            ""
        }
    }

    /// The WASI host/preview identifier this build targets, or `""` otherwise.
    #[must_use]
    pub const fn compiled_wasi_host() -> &'static str {
        if cfg!(all(target_arch = "wasm32", target_os = "wasi")) {
            if cfg!(target_env = "p2") {
                "wasip2"
            } else {
                "wasip1"
            }
        } else {
            ""
        }
    }

    /// The eBPF target version this build targets. No `cfg` target hosts a full
    /// interpreter, so this is always `""` at compile time; an eBPF evaluation
    /// is declared through the `TCL_EBPF_SPEC` environment override instead.
    #[must_use]
    pub const fn compiled_ebpf_spec() -> &'static str {
        ""
    }

    /// The environment-variable name a runtime consults to override each fact
    /// before publishing it (so a native binary can evaluate another backend's
    /// skip lists). Returns `None` for keys that are never overridden.
    #[must_use]
    pub fn override_env_var(key: &str) -> Option<&'static str> {
        match key {
            self::key::WASM => Some("TCL_WASM_SPEC"),
            self::key::WASI => Some("TCL_WASI_SPEC"),
            self::key::WASI_VERSION => Some("TCL_WASI_VERSION"),
            self::key::EBPF => Some("TCL_EBPF_SPEC"),
            _ => None,
        }
    }
}
