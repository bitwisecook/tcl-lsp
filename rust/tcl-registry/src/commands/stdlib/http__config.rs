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

//! `http::config` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::config",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set http package configuration options.",
            synopsis: &["http::config", "http::config ?options?"],
            snippet: "Without arguments returns a list of all configuration option/value pairs.  With arguments sets one or more options: ``-accept``, ``-proxyhost``, ``-proxyport``, ``-proxyfilter``, ``-urlencoding``, ``-useragent``.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}
