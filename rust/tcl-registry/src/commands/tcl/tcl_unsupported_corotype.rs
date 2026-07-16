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

//! `::tcl::unsupported::corotype` — inspect a coroutine's suspension state.
//!
//! Tcl 9 ships `::tcl::unsupported::corotype CORONAME` as part of the
//! `::tcl::unsupported::*` namespace — a documented-but-internal API
//! exposed for tooling and the tcltest harness.
//!
//! Only the fully-qualified spelling is registered (matches the
//! WASM `ns_resolve_qualified` path).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "::tcl::unsupported::corotype",
        dialects: None,
        arity: Arity::new(1, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Return the suspension state of a coroutine.",
            &["::tcl::unsupported::corotype coroName"],
            "Tcl ::tcl::unsupported::corotype (internal)",
        )),
        ..CommandSpec::DEFAULT
    }
}
