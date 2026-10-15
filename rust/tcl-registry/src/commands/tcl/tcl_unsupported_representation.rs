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

//! `::tcl::unsupported::representation` — describe a value's internal
//! representation.
//!
//! `generic/tclObj.c`'s `Tcl_RepresentationCmd`, registered from Tcl 8.6
//! (tclsh 8.5.19 has no such command; 8.6.18, 9.0.4 and 9.1.0 do). The
//! result names the value's type, reference count and address, so it differs
//! between any two objects holding the same text: it reads interpreter state
//! and is never folded.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "::tcl::unsupported::representation value",
    ..FormSpec::DEFAULT
}];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    ..SideEffect::DEFAULT
}];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        runtime_backing: RuntimeBacking::shipped(name),
        // Both engines describe the argument they were handed and run
        // nothing: `cmd_representation` (`tcl-vm/src/cmd_info.rs`) and
        // `representation_cmd` (`runtime/rust/src/cmd_info.rs`).
        traits: Traits::FRAMELESS_RUNTIME,
        surface: Some(SpecSurface::TCL86_PLUS),
        arity: Arity::new(1, 1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Describe a value's internal representation, for debugging shimmering.",
            synopsis: &["::tcl::unsupported::representation value"],
            snippet: "Reports the type the value currently holds (pure string, int, double, list, dict, …), its reference count, its address, its internal representation and the first bytes of its string representation, if it has one. Reading a value can give it a string representation or change its type, so the answer describes the value at the moment of the call; addresses and counts are the engine's own and differ from run to run. Use it to check whether a command shimmered a value.",
            source: "Tcl ::tcl::unsupported::representation (undocumented; no manual page)",
            examples: "set d [dict create a 1]\nputs [::tcl::unsupported::representation $d]\n;# -> value is a dict with a refcount of …, object pointer at …, internal representation …, no string representation",
            return_value: "A sentence naming the value's type, reference count, address, internal representation and string representation.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

pub fn spec() -> CommandSpec {
    make_spec("tcl::unsupported::representation")
}

pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::unsupported::representation")
}
