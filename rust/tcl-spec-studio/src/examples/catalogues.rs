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

//! One example per picker catalogue — what the whole vocabulary is for.

use super::{Example, focus};

pub(super) const CATALOGUE_DEFINITION_BODY: Example = Example {
    code: "oo::class create Stack {\n    method push {x} { lappend items $x }\n}",
    focuses: &[
        focus(
            0,
            "oo::class create",
            "names the grammar its body is read with",
        ),
        focus(1, "method", "a member row: its effect declares a callable"),
    ],
};
pub(super) const CATALOGUE_CONFIGURATION_REFERENCE: Example = Example {
    code: "class match -- $item equals /Common/allowed",
    focuses: &[focus(
        0,
        "/Common/allowed",
        "DataGroupComparison selects this source operand as a data-group reference",
    )],
};
/// `lsort` evaluates its comparator in the calling frame, `after` evaluates
/// its script with `TCL_EVAL_GLOBAL`, and a variable trace runs in the frame
/// of the write that fires it (Tcl 8.4 to 9.1).
pub(super) const CATALOGUE_SCRIPT_LOOKUP_SCOPE: Example = Example {
    code: "namespace eval app {\n    lsort -command by_size $files\n    after idle refresh\n    trace add variable ::count write recount\n}",
    focuses: &[
        focus(
            1,
            "by_size",
            "InvokingFrame: resolved from ::app, the namespace that calls lsort",
        ),
        focus(
            2,
            "refresh",
            "GlobalFrame: resolved from :: when the idle callback fires",
        ),
        focus(
            3,
            "recount",
            "TriggerFrame: resolved in the frame of whichever write fires the trace",
        ),
    ],
};
/// iRules header writes land on the side of the event that runs them.
pub(super) const CATALOGUE_CONNECTION_SIDE: Example = Example {
    code: "when HTTP_REQUEST {\n    HTTP::header insert X-Seen 1\n}\nwhen HTTP_RESPONSE {\n    HTTP::header insert X-Served 1\n}",
    focuses: &[
        focus(
            1,
            "HTTP::header insert",
            "Client: in HTTP_REQUEST it edits the request from the client",
        ),
        focus(
            4,
            "HTTP::header insert",
            "Server: in HTTP_RESPONSE the same write edits the response from the server",
        ),
    ],
};
