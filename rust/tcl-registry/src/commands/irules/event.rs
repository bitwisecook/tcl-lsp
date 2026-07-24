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

//! `event` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "event",
        traits: Traits::DIAGRAM_ACTION,
        // `Some(IRULES)`, not `None`: this is the F5 iRules `event`
        // (enable/disable/info of iRule event evaluation on a connection —
        // note the `ConnectionControl` side effect), not Tcl's. Core Tcl has
        // no `event` command at all — `event generate`/`add`/`info` are Tk's
        // (see `tk/event.rs`, `Some(TK)`). A catch-all `None` here made the
        // iRules command resolve under *every* dialect (plain Tcl, the EDA
        // vendors, expect, …) since `best_visible` treats `None` as visible
        // everywhere and Tk's `TK`-gated spec never intersects them — so a
        // plain-Tcl `event disable` wrongly resolved to this iRules command
        // (ConnectionControl side effect and all) instead of reading as
        // unknown. Gating to the bare `IRULES` vendor bit matches every
        // other `irules/` command and confines it to the iRules dialect.
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables or disables evaluation of the specified iRule event or all iRule events on this connection.",
            synopsis: &[
                "event info",
                "event (enable | disable) ('all')?",
                "event EVENTNAME (enable | disable)",
            ],
            snippet: "Enables or disables evaluation of the specified iRule event, or all\niRule events, on this connection. However, the iRule continues to run.\n\n**Pattern — after drop/reject**: Always follow `drop` or `reject`\nwith `event disable all` and `return` to prevent other iRules from\nrunning on the now-invalid connection.",
            source: "https://clouddocs.f5.com/api/irules/event.html",
            examples: "when HTTP_RESPONSE {\n  COMPRESS::method prefer gzip\n  event disable\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "event info",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
