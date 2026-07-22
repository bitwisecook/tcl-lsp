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
//! Ships `::tcl::unsupported::corotype CORONAME` as part of the
//! `::tcl::unsupported::*` namespace — a documented-but-internal API
//! exposed for tooling and the tcltest harness. Coroutines are a Tcl 8.6
//! feature (TIP 396), and `corotype` alongside them — empirically
//! confirmed present against a real tclsh 8.6.14, not 9.0-only as an
//! earlier version of this comment claimed.
//!
//! The VM registers only the fully-qualified spelling and relies on its
//! own generic bare→qualified namespace resolution (`ns_resolve_qualified`)
//! to reach it either way. `tcl-registry`'s lookup has no such generic
//! fallback in that direction (only qualified→bare, the opposite way), so —
//! matching the `tcl::process`/`tcl::zipfs`/`tcl::idna` sibling 9.0-era
//! additions — both spellings are registered here explicitly.
use crate::prelude::*;

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TCL86_PLUS),
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

/// Command spec for the namespace-relative form `tcl::unsupported::corotype`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::unsupported::corotype")
}

/// Command spec for the fully-qualified form `::tcl::unsupported::corotype`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::unsupported::corotype")
}
