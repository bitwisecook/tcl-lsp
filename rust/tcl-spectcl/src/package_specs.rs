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

//! The packs a package ships: its manifest's `spec` directive resolved against
//! the package directory, and the hash that names each pack.
//!
//! A package that carries `spec { packs {…} }` lists its `.tclspec` files, and
//! discovery ([`crate::discovery`]) loads those beside its manifest and no
//! others. The lockfile records the same packs by the content hash a compiled
//! unit's claim on each carries ([`pack_file_hash`]), so the package manager
//! and a runtime identify a pack by one value rather than by two hashing rules.

use std::path::{Path, PathBuf};

use tcl_lsp_core::vfs::SourceStore;
use tcl_pkg_model::lockfile::format_spec_integrity;
use tcl_pkg_model::manifest::SpecDirective;

pub use crate::loader::pack_file_hash;

/// The packs `directive` names, as paths under the package directory `dir`, in
/// the order the manifest names them. Every name is relative and inside the
/// package (the manifest refuses any other), so the join cannot leave `dir`.
#[must_use]
pub fn pack_paths(dir: &Path, directive: &SpecDirective) -> Vec<PathBuf> {
    directive.packs.iter().map(|pack| dir.join(pack)).collect()
}

/// What a lockfile records for the packs the package at `dir` ships:
/// [`pack_file_hash`] of each pack `directive` names, in the order it names
/// them, as `tcl_pkg_model::lockfile::format_spec_integrity` writes them.
///
/// # Errors
///
/// A pack the directive names cannot be read; the message names the file.
pub fn spec_integrity(
    store: &dyn SourceStore,
    dir: &Path,
    directive: &SpecDirective,
) -> std::io::Result<String> {
    let mut hashes = Vec::with_capacity(directive.packs.len());
    for path in pack_paths(dir, directive) {
        let source = store
            .read_to_string(&path)
            .map_err(|err| std::io::Error::new(err.kind(), format!("{}: {err}", path.display())))?;
        hashes.push(pack_file_hash(&path, &source));
    }
    Ok(format_spec_integrity(&hashes))
}
