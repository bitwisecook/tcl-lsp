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

//! Authoritative field inventory for durable compiler units.
//!
//! Each inventory and exhaustive field witness share a declaration in the owner
//! crate, so private retained inputs are covered without exposing their values.
//! Consumers must explicitly account for every published field name; these
//! names grant no source, configuration, or native execution authority.

pub use crate::compilation_unit::durable_field_inventory::{
    COMPILATION_UNIT_FIELDS, FUNCTION_UNIT_FIELDS, assert_durable_field_inventory,
};
