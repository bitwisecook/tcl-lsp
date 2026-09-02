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

//! Command specification modules — one directory per dialect.
//!
//! The EDA vendor libraries are **not** here: `sdc_base` and the five vendor
//! packs ship as bundled `.tclspec` loadables under `specs/`, loaded by
//! `tcl-spectcl` (`docs/design/spec-packs.md`, "the EDA vendor libraries ship
//! as bundled `.tclspec` loadables … so the loader path is exercised in
//! production from day one").

pub mod argparse;
pub mod bpf;
pub mod expect;
pub mod iapps;
pub mod irules;
pub mod itcl;
pub mod spectcl;
pub mod sslictcl;
pub mod stdlib;
pub mod tcl;
pub mod tcllib;
pub mod ticklecharts;
pub mod tk;
