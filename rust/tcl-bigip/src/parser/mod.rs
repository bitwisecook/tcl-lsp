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

//! BIG-IP config parser.

pub mod bespoke;
pub mod driver;
pub mod header;
pub mod helpers;
pub mod scalar;

pub use driver::{BigipConfig, Placed, parse_bigip_conf};

pub use header::{ObjectTypeIndex, parse_generic_header};
pub use helpers::{
    Block, Property, extract_blocks, parse_keyed_block_entries, parse_list_block, parse_properties,
    parse_properties_with_spans, tokenise_header,
};
