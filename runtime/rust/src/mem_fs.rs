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

//! A small in-memory [`Filesystem`] — the VFS the WASM hosts mount so the
//! filesystem-backed startup path (`source`, `package require`, `glob`, `file`)
//! works on a target with no host filesystem (`wasm32-wasip1`/browser).
//!
//! The Tcl standard library (`init.tcl`, the `tcltest` package, the package
//! indices) is **embedded in the binary** at build time
//! ([`crate::embedded_stdlib`]) and seeded into this VFS, so a self-contained
//! WASM module bootstraps the real stdlib with no `--dir` preopen and no source
//! files shipped alongside it.
//!
//! Paths are POSIX-style and normalised (`.`/`..`/`//` collapsed) to an absolute
//! key. Directories are implicit — a directory exists when it is the prefix of a
//! stored file or was created explicitly (so `glob`/`read_dir` over the mount
//! point enumerate the package subdirectories). Writes are supported (tcltest's
//! temp files), backed by the same map.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use tcl_platform::{Filesystem, HostError, Metadata};

/// An in-memory filesystem: a path → bytes map with synthesised directories.
#[derive(Default)]
pub struct MemFs {
    files: RefCell<BTreeMap<String, Vec<u8>>>,
    /// Explicitly created directories (those not implied by a stored file).
    dirs: RefCell<BTreeSet<String>>,
}

/// Normalise `path` to an absolute, slash-separated key: collapse `//`, drop `.`,
/// resolve `..`, and strip any trailing slash. The root is `"/"`.
fn norm(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    let mut s = String::from("/");
    s.push_str(&out.join("/"));
    s
}

/// The directory prefix used to test "is an entry directly under `dir`": `"/"` for
/// the root, else `dir + "/"`.
fn child_prefix(dir: &str) -> String {
    if dir == "/" {
        "/".to_string()
    } else {
        format!("{dir}/")
    }
}

impl MemFs {
    /// An empty VFS.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert (or overwrite) a file at `path` with `data`. Parent directories are
    /// implied by the key, so they need no explicit creation.
    pub fn insert(&self, path: &str, data: impl Into<Vec<u8>>) {
        self.files.borrow_mut().insert(norm(path), data.into());
    }

    /// Whether `path` is a directory: the root, an explicitly created directory,
    /// or the prefix of a stored file.
    fn is_dir(&self, path: &str) -> bool {
        if path == "/" || self.dirs.borrow().contains(path) {
            return true;
        }
        let prefix = child_prefix(path);
        self.files.borrow().keys().any(|k| k.starts_with(&prefix))
            || self.dirs.borrow().iter().any(|d| d.starts_with(&prefix))
    }
}

impl Filesystem for MemFs {
    fn exists(&self, path: &str) -> bool {
        let p = norm(path);
        self.files.borrow().contains_key(&p) || self.is_dir(&p)
    }

    fn metadata(&self, path: &str) -> Result<Metadata, HostError> {
        let p = norm(path);
        if let Some(bytes) = self.files.borrow().get(&p) {
            return Ok(Metadata {
                is_dir: false,
                is_file: true,
                is_symlink: false,
                executable: false,
                len: bytes.len() as u64,
                mtime_secs: 0,
                dev: 0,
                ino: 0,
                nlink: 1,
                uid: 0,
                gid: 0,
                mode: 0o100644,
                blocks: 0,
                blksize: 0,
                atime_secs: 0,
                ctime_secs: 0,
            });
        }
        if self.is_dir(&p) {
            return Ok(Metadata {
                is_dir: true,
                is_file: false,
                is_symlink: false,
                executable: true,
                len: 0,
                mtime_secs: 0,
                dev: 0,
                ino: 0,
                nlink: 1,
                uid: 0,
                gid: 0,
                mode: 0o40755,
                blocks: 0,
                blksize: 0,
                atime_secs: 0,
                ctime_secs: 0,
            });
        }
        Err(HostError::NotFound)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, HostError> {
        self.files
            .borrow()
            .get(&norm(path))
            .cloned()
            .ok_or(HostError::NotFound)
    }

    fn write(&self, path: &str, data: &[u8]) -> Result<(), HostError> {
        self.files.borrow_mut().insert(norm(path), data.to_vec());
        Ok(())
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, HostError> {
        let p = norm(path);
        if !self.is_dir(&p) {
            return Err(HostError::NotFound);
        }
        let prefix = child_prefix(&p);
        let mut names = BTreeSet::new();
        let take_child = |key: &str, names: &mut BTreeSet<String>| {
            if let Some(rest) = key.strip_prefix(&prefix) {
                if !rest.is_empty() {
                    // The immediate child is the first path segment of `rest`.
                    let child = rest.split('/').next().unwrap_or(rest);
                    names.insert(child.to_string());
                }
            }
        };
        for key in self.files.borrow().keys() {
            take_child(key, &mut names);
        }
        for dir in self.dirs.borrow().iter() {
            take_child(dir, &mut names);
        }
        Ok(names.into_iter().collect())
    }

    fn create_dir_all(&self, path: &str) -> Result<(), HostError> {
        let p = norm(path);
        let mut acc = String::new();
        for seg in p.split('/').filter(|s| !s.is_empty()) {
            acc.push('/');
            acc.push_str(seg);
            self.dirs.borrow_mut().insert(acc.clone());
        }
        Ok(())
    }

    fn create_dir(&self, path: &str) -> Result<(), HostError> {
        let p = norm(path);
        if self.exists(&p) {
            return Err(HostError::AlreadyExists);
        }
        let parent = p
            .rsplit_once('/')
            .map_or("/", |(head, _)| if head.is_empty() { "/" } else { head });
        if !self.is_dir(parent) {
            return Err(HostError::NotFound);
        }
        self.dirs.borrow_mut().insert(p);
        Ok(())
    }

    fn remove(&self, path: &str, recursive: bool) -> Result<(), HostError> {
        let p = norm(path);
        if self.files.borrow_mut().remove(&p).is_some() {
            return Ok(());
        }
        if self.is_dir(&p) {
            if !recursive {
                // Non-recursive removal of a non-empty directory fails.
                if !self.read_dir(&p)?.is_empty() {
                    return Err(HostError::Io("directory not empty".to_string()));
                }
            }
            let prefix = child_prefix(&p);
            self.files
                .borrow_mut()
                .retain(|k, _| k != &p && !k.starts_with(&prefix));
            self.dirs
                .borrow_mut()
                .retain(|d| d != &p && !d.starts_with(&prefix));
            return Ok(());
        }
        Err(HostError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_collapses() {
        assert_eq!(norm("/a//b/./c"), "/a/b/c");
        assert_eq!(norm("/a/b/../c"), "/a/c");
        assert_eq!(norm("/"), "/");
        assert_eq!(norm("/a/"), "/a");
    }

    #[test]
    fn files_and_implied_dirs() {
        let fs = MemFs::new();
        fs.insert("/tcl/library/init.tcl", b"# init".to_vec());
        fs.insert("/tcl/library/tcltest/tcltest.tcl", b"# tcltest".to_vec());

        assert!(fs.exists("/tcl/library/init.tcl"));
        assert!(fs.exists("/tcl/library"));
        assert!(fs.exists("/tcl/library/tcltest"));
        assert!(fs.metadata("/tcl/library").unwrap().is_dir);
        assert!(fs.metadata("/tcl/library/init.tcl").unwrap().is_file);
        assert_eq!(fs.read("/tcl/library/init.tcl").unwrap(), b"# init");

        let mut entries = fs.read_dir("/tcl/library").unwrap();
        entries.sort();
        assert_eq!(entries, vec!["init.tcl", "tcltest"]);
    }

    #[test]
    fn write_then_remove() {
        let fs = MemFs::new();
        fs.write("/tmp/x", b"hi").unwrap();
        assert_eq!(fs.read("/tmp/x").unwrap(), b"hi");
        fs.remove("/tmp/x", false).unwrap();
        assert!(!fs.exists("/tmp/x"));
    }
}
