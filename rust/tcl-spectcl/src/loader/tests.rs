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

use tcl_dialect::model::{Family, SpecProvider, SurfaceQuery, surface_admits};

use tcl_dialect::BracedVarStyle;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::{BuildProfileId, EnvironmentKind, Provenance, Release, WorldPolicy};

use super::*;

mod available;
mod block_descriptors;
mod body_rows;
mod command;
mod dialect_block;
mod environment_block;
mod lifecycles;
mod pack_scope;
mod role_resolvers;
mod value_parsers;
mod vocabularies;
mod vocabulary;
