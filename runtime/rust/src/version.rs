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

//! Runtime Tcl version facts shared by interpreter startup, `package`, and
//! `tcl::build-info`.
//!
//! Nothing here is a constant any more (ledger row B4). This interpreter's
//! emulated release is settable — `set_dialect_profile` / `--tcl-version` —
//! and a build identity or a `package provide Tcl` frozen at `9.0.4` was
//! simply wrong under every other pin, as well as being a second table that
//! could disagree with `tcl-vm`'s. Both engines now read `tcl_dialect`'s
//! release vocabulary.

use tcl_dialect::TclVersion;

/// The engine word this runtime contributes to its `::tcl::build-info`
/// string, where C names its compiler and build options.
pub(crate) const BUILD_INFO_ENGINE: &str = "rust";

/// This interpreter's `::tcl::build-info` string for the release it is
/// pinned to.
pub(crate) fn build_info(version: TclVersion) -> String {
    tcl_dialect::build_info::build_info(version, BUILD_INFO_ENGINE)
}
