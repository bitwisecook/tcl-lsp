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

//! `next` — the explicit, non-terminal continuation outcome for handler
//! composition (issue #1204). A handler that ends a path with `next` yields no
//! decision, so the next handler in priority order runs; if every handler on a
//! path yields `next`, the event's declared default verdict applies.
//!
//! `next` is a verdict for lowering purposes (it ends the handler's program by
//! returning a reserved continuation sentinel), but the composition model reads
//! that sentinel as "continue the chain" rather than a packet decision. It is
//! valid in every program type.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

pub fn spec() -> CommandSpec {
    const OP: BpfOpSpec = BpfOpSpec::verdict(BpfVerdictKind::Next, BpfProgTypeSet::ALL);
    CommandSpec {
        name: "next",
        surface: Some(SpecSurface::BPF),
        arity: Arity::exact(0),
        bpf_op: Some(&OP),
        ..CommandSpec::DEFAULT
    }
}
