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

//! One example per form-group heading — what the whole group is about.

use super::{Example, focus};

pub(super) const IDENTITY: Example = Example {
    code: "mycommand value",
    focuses: &[focus(0, "mycommand", "describes the command word")],
};
pub(super) const AVAILABILITY: Example = Example {
    code: "mycommand value",
    focuses: &[focus(
        0,
        "mycommand",
        "decides whether this command exists here",
    )],
};
pub(super) const ARGUMENTS: Example = Example {
    code: "mycommand first second",
    focuses: &[focus(0, "first", "describes this argument position")],
};
pub(super) const TYPES: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "describes values flowing through this call",
    )],
};
pub(super) const SUBCOMMANDS: Example = Example {
    code: "mycommand action value",
    focuses: &[focus(0, "action", "selects the subcommand specification")],
};
pub(super) const DOCUMENTATION: Example = Example {
    code: "mycommand -mode fast value",
    focuses: &[focus(
        0,
        "mycommand -mode fast value",
        "is the invocation readers see documented",
    )],
};
pub(super) const OPTIONS: Example = Example {
    code: "mycommand -mode fast value",
    focuses: &[focus(
        0,
        "-mode fast",
        "describes the option and its value words",
    )],
};
pub(super) const BEHAVIOUR: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "describes the behaviour of the whole invocation",
    )],
};
pub(super) const EFFECTS: Example = Example {
    code: "set result [mycommand $state]",
    focuses: &[focus(
        0,
        "[mycommand $state]",
        "records state read or changed by this invocation",
    )],
};
pub(super) const HOOKS: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "selects special handling for this invocation",
    )],
};
pub(super) const TAINT: Example = Example {
    code: "set safe [mycommand $untrusted]\nputs $safe",
    focuses: &[
        focus(
            0,
            "[mycommand $untrusted]",
            "colours or checks data at this call",
        ),
        focus(1, "$safe", "the resulting proof follows this value"),
    ],
};
pub(super) const DEPRECATION: Example = Example {
    code: "oldcommand value",
    focuses: &[focus(
        0,
        "oldcommand",
        "reports or translates this deprecated invocation",
    )],
};
pub(super) const ADVANCED: Example = Example {
    code: "mycommand $value",
    focuses: &[focus(
        0,
        "mycommand $value",
        "applies custom registry behaviour to this call",
    )],
};
