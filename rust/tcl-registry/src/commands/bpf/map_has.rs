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

//! `map_has` — key-presence test (`map_has DST NAME {KEY}`): `DST` becomes 1
//! when the key is present, 0 when absent — the way to distinguish a missing
//! key from a stored zero (`map_get` folds both to 0).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    const OP: BpfOpSpec = BpfOpSpec::gated(BpfOpKind::MapHas, BpfEffects::MAP_READ);
    CommandSpec {
        name: "map_has",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        bpf_op: Some(&OP),
        ..CommandSpec::DEFAULT
    }
}
