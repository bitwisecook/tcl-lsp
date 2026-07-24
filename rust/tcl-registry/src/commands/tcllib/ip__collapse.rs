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

//! `ip::collapse` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::collapse addressList",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::collapse",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Collapse a list of IP addresses or subnets into the minimal set.",
            synopsis: &["ip::collapse addressList"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "A list of collapsed address ranges.",
        }),
        forms: FORMS,
        tcllib_package: Some("ip"),
        required_package: Some("ip"),
        ..CommandSpec::DEFAULT
    }
}
