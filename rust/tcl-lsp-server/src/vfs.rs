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

//! Where the server's *closed* files come from — see [`tcl_lsp_core::vfs`],
//! which owns the seam and documents it.
//!
//! [`Backend`] holds one [`SourceStore`]: [`NativeStore`] natively, a
//! host-filled [`MemoryStore`] in a browser worker. The trait lives one crate
//! down because two of the paths routed through it do too — the package
//! database in [`tcl_lsp_core::package_resolver`] and `.tclspec` discovery in
//! [`tcl_spectcl::discovery`] — and this module keeps `crate::vfs::…` working
//! as the server's own spelling of it.
//!
//! [`Backend`]: crate::Backend

pub use tcl_lsp_core::vfs::{DirEntry, MemoryStore, Metadata, NativeStore, SourceStore};

/// The store path a host puts its own `.tclspec` packs under, re-exported here
/// because it is part of *this* crate's contract with a host: a browser worker
/// has no executable for [`tcl_spectcl::discovery::bundled_dir`] to look beside,
/// so the bundled tier is read from the store at this prefix instead.
pub use tcl_spectcl::discovery::VIRTUAL_PACK_MOUNT;
