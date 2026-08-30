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

//! `when` — attach a handler to a BPF event (`when EVENT ?priority N? { body }`).
//!
//! Unlike the F5 iRules `when`, this spec carries **no** `LoweringHookId::When`,
//! so the BPF-Tcl front-end keeps it a generic call and re-lowers the body in
//! its own (BPF-native) event space.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

pub fn spec() -> CommandSpec {
    const OP: BpfOpSpec = BpfOpSpec::framework(BpfDeclKind::When);
    CommandSpec {
        name: "when",
        surface: Some(SpecSurface::BPF),
        // EVENT BODY  |  EVENT priority N BODY
        arity: Arity::new(2, 4),
        bpf_op: Some(&OP),
        ..CommandSpec::DEFAULT
    }
}
