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

//! `LB::queue` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::queue",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns queue information.",
            synopsis: &[
                "LB::queue queued",
                "LB::queue on",
                "LB::queue limit",
                "LB::queue depth",
            ],
            snippet: "Returns queue information. Connection queuing details:\n\n    * Operates at the TCP level\n    * Only engages when the connection limit is hit\n    * Queue is specified by length, time, or both (in the pool configuration)\n    * Queues operate per-tmm, there is no state sharing\n        * Length limit divided by tmm count\n        * FIFO guarantees only per-tmm\n    * Queued at the pool level for non-persistent connections\n    * Queued at the pool member level for persistent connections.",
            source: "https://clouddocs.f5.com/api/irules/LB__queue.html",
            examples: "when LB_QUEUED {\n    log local0. \"[IP::local_addr] was queued - [LB::queue depth one pool1] / [LB::queue limit depth pool1]\"\n}",
            return_value: "LB::queue limit depth|time [<pool name>] Returns queue limit info (depth is per-tmm)",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::queue queued",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
