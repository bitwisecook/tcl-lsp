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

//! Where the server's *closed* files come from.
//!
//! Everything the editor has open lives in the server's document store and
//! never touches this module. What does touch it is the other half of the
//! picture: the file a `source` edge points at, the sibling the workspace scan
//! indexed, the `config.ini` a session layers under the editor's settings, the
//! `pkgIndex.tcl` the package database parsed, the `.tclspec` a pack was loaded
//! from. Natively those are all `std::fs` calls. In a browser worker there is
//! no `std::fs` to call — the host has the bytes and has to hand them over.
//!
//! [`SourceStore`] is that seam, and `tcl_lsp_server::Backend` holds exactly
//! one of them.
//!
//! * [`NativeStore`] is the native implementation and is a literal delegation
//!   to `std::fs` — the same calls, in the same order, returning the same
//!   `io::Error`s the call sites used to make for themselves. A native server
//!   built with this module behaves byte-identically to one built without it;
//!   that is the whole specification of `NativeStore`, and any cleverness
//!   added to it (caching, path rewriting, normalisation) breaks it.
//! * [`MemoryStore`] is the browser implementation: a `PathBuf`-keyed byte map
//!   the host fills in over `postMessage`, plus a URI alias table so a file
//!   registered under the client's spelling of a URI is still found when the
//!   server re-derives a path from its own canonical spelling. A path the host
//!   never supplied reads as [`io::ErrorKind::NotFound`], which every call
//!   site here already treats as "that file contributes nothing" — the same
//!   answer a native server gives for a file that genuinely is not there.
//!
//! # Why the trait lives here
//!
//! It was born in `tcl-lsp-server`, where every call site was. Finishing the
//! seam moved the whole-workspace paths onto it, and two of those live below
//! the server: the package database ([`crate::package_resolver`]) and
//! `.tclspec` discovery (`tcl_spectcl::discovery`). `tcl-lsp-core` is the
//! lowest crate that can hold the trait *and both implementations* undivided —
//! [`SourceStore::read_source`] needs [`crate::source_decode`], so a home
//! below this crate would split the decode default off the trait.
//! `tcl-core-types`, the other leaf both dependants share, is `#![no_std]` and
//! cannot name `std::fs` at all; `tcl-platform` bans syscalls by charter and
//! already owns a host-filesystem trait, so a second one there would put two
//! owners of one axis in one crate. `tcl-spectcl` therefore depends on this
//! crate for the trait, and `tcl_lsp_server::vfs` re-exports it so the server's
//! own call sites did not move.
//!
//! # Scope
//!
//! Every path the server reads a closed file on now goes through the store:
//! the single-document chokepoints (the closed-file read behind
//! `read_document`, the decode-report comparison, `scan_disk_file`, the
//! `config.ini` layers, the `exportConfig` write, the notice line-range read,
//! the recovery-widening package read) **and** the whole-workspace ones — the
//! workspace folder scan, the package database
//! ([`scan_path_in`](crate::package_resolver::PackageResolver::scan_path_in) /
//! [`scan_tree_in`](crate::package_resolver::PackageResolver::scan_tree_in)),
//! the W120 / W123 package scans, and spec-pack discovery. A host that fills a
//! [`MemoryStore`] gets a real workspace session, not an empty one.
//!
//! The store-free spellings (`scan_path`, `scan_tree`,
//! `tcl_spectcl::discover`, `bundled::load_discovered`) survive as
//! [`NativeStore`] wrappers, the same shape `Backend::new` and
//! `Backend::with_store` use — native callers and their tests did not move, so
//! the store-taking form cannot change native behaviour by accident.
//!
//! What is deliberately still `std::fs`, because each is a capability a byte
//! map genuinely does not have: the compiled-pack disk cache
//! (`tcl_spectcl::cache`, a native-only speed-up that already no-ops when its
//! directory is unwritable), `std::env::current_exe`-relative bundled-pack
//! discovery and its `canonicalize` (see
//! `tcl_spectcl::discovery::VIRTUAL_PACK_MOUNT` for the store path that
//! replaces the former), and Tcl-installation discovery
//! ([`crate::tcl_install`], which probes for a native interpreter a browser has
//! no equivalent of).
//!
//! `docs/design/contracts/lsp-source-store.md` is the full contract.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What a directory listing reports about one entry.
///
/// Deliberately smaller than `std::fs::DirEntry`: the callers only ever ask
/// for the path and the entry's kind, and a browser store has no honest answer
/// for anything more.
///
/// `is_dir` and `is_file` are the two questions `std::fs::FileType` answers,
/// and they are **both false** for a symlink — the walks that consult them
/// (the workspace scan, the package-index scan, `.tclspec` discovery) all
/// asked `file_type().is_dir()` / `.is_file()` before this trait existed, so a
/// symlink was neither descended into nor indexed. Collapsing them to one flag
/// would silently start indexing symlinked sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// The entry's full path.
    pub path: PathBuf,
    /// Whether the entry is a directory, not following a symlink.
    pub is_dir: bool,
    /// Whether the entry is a regular file, not following a symlink.
    pub is_file: bool,
}

/// The subset of `std::fs::Metadata` this crate actually consults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    /// Whether the path names a directory.
    pub is_dir: bool,
    /// The file's length in bytes (zero for a directory).
    pub len: u64,
}

/// The source of every file the server reads that the editor has not opened.
///
/// Implementations must be cheap to share (`Backend` holds an `Arc<dyn
/// SourceStore>` and clones it into blocking tasks) and must report a missing
/// path as [`io::ErrorKind::NotFound`] rather than panicking.
pub trait SourceStore: std::fmt::Debug + Send + Sync {
    /// Read a file's bytes.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error; a path the store does not hold is
    /// [`io::ErrorKind::NotFound`].
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Read a file as UTF-8 text.
    ///
    /// # Errors
    ///
    /// As [`Self::read`], plus [`io::ErrorKind::InvalidData`] for bytes that
    /// are not valid UTF-8 — matching `std::fs::read_to_string`. Callers that
    /// must tolerate an ill-formed byte use [`Self::read_source`] instead.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// List a directory's immediate entries, in unspecified order.
    ///
    /// An entry that cannot itself be read is **skipped**, not reported: every
    /// caller in the tree wrote `std::fs::read_dir(dir)?.flatten()`, so a
    /// mid-listing error costs that one entry and never the whole directory.
    ///
    /// `is_dir` is the entry's own type and does not follow a symlink —
    /// `std::fs::DirEntry::file_type`, not a `stat` of the target. A caller
    /// that wants the target's kind asks [`Self::is_dir`] with the entry path.
    ///
    /// # Errors
    ///
    /// Propagates the error of *opening* the directory.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>>;

    /// Report what a path is.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error; a path the store does not hold is
    /// [`io::ErrorKind::NotFound`].
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;

    /// Replace a file's contents, creating it if needed.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error.
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;

    /// Create a directory and every missing parent.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Whether the path names a readable file.
    fn is_file(&self, path: &Path) -> bool {
        self.metadata(path).is_ok_and(|m| !m.is_dir)
    }

    /// Whether the path names a readable directory.
    fn is_dir(&self, path: &Path) -> bool {
        self.metadata(path).is_ok_and(|m| m.is_dir)
    }

    /// Whether anything exists at the path.
    fn exists(&self, path: &Path) -> bool {
        self.metadata(path).is_ok()
    }

    /// Read a file through the shared lossy decoder.
    ///
    /// The counterpart of [`crate::source_decode::read_source_file`], and the
    /// reason it exists (issue #1326): a closed file with one ill-formed byte
    /// must still contribute its procs, its `source` edges, and its symbols,
    /// so decoding is lossy and only the *read* can fail.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error. Decoding itself never fails.
    fn read_source(&self, path: &Path) -> io::Result<(String, crate::source_decode::DecodeReport)> {
        Ok(crate::source_decode::decode_source(&self.read(path)?))
    }
}

/// The real filesystem — a direct delegation to `std::fs`.
///
/// Every method is the call the corresponding site made before this trait
/// existed. Nothing is cached, rewritten, or normalised: the native server's
/// behaviour is defined to be `std::fs`'s.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeStore;

impl SourceStore for NativeStore {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(path)?.flatten() {
            let kind = entry.file_type();
            out.push(DirEntry {
                path: entry.path(),
                is_dir: kind.as_ref().is_ok_and(std::fs::FileType::is_dir),
                is_file: kind.as_ref().is_ok_and(std::fs::FileType::is_file),
            });
        }
        Ok(out)
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let meta = std::fs::metadata(path)?;
        Ok(Metadata {
            is_dir: meta.is_dir(),
            len: meta.len(),
        })
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        std::fs::write(path, contents)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }
}

/// An in-memory filesystem the host fills in.
///
/// Two tables. `files` maps a path to its bytes and is what every
/// [`SourceStore`] method consults. `aliases` maps a URI string to the path it
/// was registered under, because the two spellings of one file — the client's
/// URI and the path the server re-derives from its own canonical URI — are not
/// guaranteed to round-trip through `Uri::to_file_path` on every platform. A
/// host that registers content by URI can therefore always find it again.
///
/// Directories are implicit: a path is a directory exactly when some stored
/// file is under it, which is enough for [`SourceStore::read_dir`] and
/// [`SourceStore::is_dir`] and avoids the host having to declare a tree.
#[derive(Debug, Default)]
pub struct MemoryStore {
    files: Mutex<HashMap<PathBuf, Vec<u8>>>,
    aliases: Mutex<HashMap<String, PathBuf>>,
}

impl MemoryStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace a file's bytes.
    pub fn upsert(&self, path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) {
        let path = path.into();
        Self::lock(&self.files).insert(path, contents.into());
    }

    /// Add or replace a file's bytes, and record `uri` as another name for it.
    pub fn upsert_uri(&self, uri: &str, path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) {
        let path = path.into();
        Self::lock(&self.aliases).insert(uri.to_owned(), path.clone());
        Self::lock(&self.files).insert(path, contents.into());
    }

    /// Forget a file. Returns whether anything was stored under `path`.
    pub fn remove(&self, path: &Path) -> bool {
        Self::lock(&self.files).remove(path).is_some()
    }

    /// Forget the file registered under `uri`, whatever path it was stored at.
    /// Returns whether anything was removed.
    pub fn remove_uri(&self, uri: &str) -> bool {
        let path = Self::lock(&self.aliases).remove(uri);
        path.is_some_and(|p| self.remove(&p))
    }

    /// The path `uri` was registered under, if any.
    #[must_use]
    pub fn path_for_uri(&self, uri: &str) -> Option<PathBuf> {
        Self::lock(&self.aliases).get(uri).cloned()
    }

    /// How many files the store holds.
    #[must_use]
    pub fn len(&self) -> usize {
        Self::lock(&self.files).len()
    }

    /// Whether the store holds no files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Take a lock, recovering from a poisoned one.
    ///
    /// A poisoned lock here means a panic while the map was borrowed. The map
    /// is a plain `HashMap` with no cross-entry invariant, so the worst a
    /// poisoning can have left behind is a half-finished insert — nothing that
    /// makes the remaining entries wrong. Refusing to serve any file at all
    /// after one panic would be the larger failure.
    fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn not_found(path: &Path) -> io::Error {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not in the in-memory source store", path.display()),
        )
    }
}

impl SourceStore for MemoryStore {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        Self::lock(&self.files)
            .get(path)
            .cloned()
            .ok_or_else(|| Self::not_found(path))
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        String::from_utf8(self.read(path)?)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let files = Self::lock(&self.files);
        let mut seen: HashMap<PathBuf, bool> = HashMap::new();
        for stored in files.keys() {
            let Ok(rest) = stored.strip_prefix(path) else {
                continue;
            };
            let mut parts = rest.components();
            let Some(first) = parts.next() else { continue };
            let child = path.join(first.as_os_str());
            let is_dir = parts.next().is_some();
            seen.entry(child)
                .and_modify(|d| *d = *d || is_dir)
                .or_insert(is_dir);
        }
        if seen.is_empty() && !files.keys().any(|stored| stored.starts_with(path)) {
            return Err(Self::not_found(path));
        }
        Ok(seen
            .into_iter()
            .map(|(path, is_dir)| DirEntry {
                path,
                is_dir,
                is_file: !is_dir,
            })
            .collect())
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let files = Self::lock(&self.files);
        if let Some(bytes) = files.get(path) {
            return Ok(Metadata {
                is_dir: false,
                len: bytes.len() as u64,
            });
        }
        if files.keys().any(|stored| stored.starts_with(path)) {
            return Ok(Metadata {
                is_dir: true,
                len: 0,
            });
        }
        Err(Self::not_found(path))
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        self.upsert(path, contents.to_vec());
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        // Directories are implied by the files under them, so there is nothing
        // to create and nothing that can fail.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryStore, NativeStore, SourceStore as _};
    use std::path::Path;

    #[test]
    fn a_stored_file_reads_back() {
        let store = MemoryStore::new();
        store.upsert("/w/a.tcl", b"puts hi\n".to_vec());
        assert_eq!(
            store.read_to_string(Path::new("/w/a.tcl")).expect("stored"),
            "puts hi\n"
        );
        assert!(store.is_file(Path::new("/w/a.tcl")));
        assert!(!store.is_dir(Path::new("/w/a.tcl")));
    }

    #[test]
    fn a_missing_file_is_not_found_rather_than_a_panic() {
        let store = MemoryStore::new();
        let err = store.read(Path::new("/w/gone.tcl")).expect_err("missing");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn an_ill_formed_byte_still_decodes_through_read_source() {
        let store = MemoryStore::new();
        store.upsert("/w/latin.tcl", vec![b'p', 0xFF, b'\n']);
        let (text, report) = store
            .read_source(Path::new("/w/latin.tcl"))
            .expect("stored");
        assert!(text.contains('p'));
        assert!(!report.is_faithful());
    }

    #[test]
    fn a_uri_alias_finds_the_file_again() {
        let store = MemoryStore::new();
        store.upsert_uri("file:///w/a.tcl", "/w/a.tcl", b"set x 1\n".to_vec());
        assert_eq!(
            store.path_for_uri("file:///w/a.tcl").expect("alias"),
            Path::new("/w/a.tcl")
        );
        assert!(store.remove_uri("file:///w/a.tcl"));
        assert!(!store.exists(Path::new("/w/a.tcl")));
    }

    #[test]
    fn directories_are_implied_by_the_files_under_them() {
        let store = MemoryStore::new();
        store.upsert("/w/lib/a.tcl", b"".to_vec());
        store.upsert("/w/b.tcl", b"".to_vec());
        assert!(store.is_dir(Path::new("/w")));
        let mut names: Vec<_> = store
            .read_dir(Path::new("/w"))
            .expect("listing")
            .into_iter()
            .map(|e| (e.path, e.is_dir))
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                (Path::new("/w/b.tcl").to_path_buf(), false),
                (Path::new("/w/lib").to_path_buf(), true),
            ]
        );
    }

    #[test]
    fn a_write_is_readable_again() {
        let store = MemoryStore::new();
        store
            .write(Path::new("/cfg/config.ini"), b"[features]\n")
            .expect("write");
        assert_eq!(
            store
                .read_to_string(Path::new("/cfg/config.ini"))
                .expect("written"),
            "[features]\n"
        );
    }

    #[test]
    fn the_native_store_is_std_fs() {
        let dir = std::env::temp_dir().join("tcl-lsp-vfs-native-store");
        let store = NativeStore;
        store.create_dir_all(&dir).expect("mkdir");
        let file = dir.join("probe.tcl");
        store.write(&file, b"proc p {} {}\n").expect("write");
        assert_eq!(store.read_to_string(&file).expect("read"), "proc p {} {}\n");
        assert!(store.is_dir(&dir));
        assert!(
            store
                .read_dir(&dir)
                .expect("listing")
                .iter()
                .any(|e| e.path == file)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
