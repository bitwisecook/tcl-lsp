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

pub(super) const CATALOGUE_ARG_ROLE: Example = Example {
    code: "mycommand ARGUMENT",
    focuses: &[focus(0, "ARGUMENT", "classifies this argument word")],
};
pub(super) const CATALOGUE_TYPE: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(0, "$value", "describes the value at this position")],
};
pub(super) const CATALOGUE_PRESENTATION: Example = Example {
    code: "mycommand {script body}",
    focuses: &[focus(
        0,
        "{script body}",
        "controls how this script is laid out",
    )],
};
pub(super) const CATALOGUE_EFFECT: Example = Example {
    code: "mycommand $state",
    focuses: &[focus(
        0,
        "mycommand $state",
        "classifies the effect of this invocation",
    )],
};
pub(super) const CATALOGUE_HOOK: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "selects special handling for this call",
    )],
};
pub(super) const CATALOGUE_TAINT: Example = Example {
    code: "set checked [validate $user_input]\nmy_sink $checked",
    focuses: &[
        focus(0, "$user_input", "starts as data that may be untrusted"),
        focus(1, "$checked", "carries the selected proof into this sink"),
    ],
};
pub(super) const CATALOGUE_DIALECT: Example = Example {
    code: "mycommand value",
    focuses: &[focus(
        0,
        "mycommand",
        "is available in the selected language surface",
    )],
};
pub(super) const CATALOGUE_OPTION: Example = Example {
    code: "mycommand -option VALUE",
    focuses: &[focus(
        0,
        "-option VALUE",
        "controls these option value words",
    )],
};
pub(super) const CATALOGUE_PREFIX: Example = Example {
    code: "mycommand callback\nproc callback {appended args} { ... }",
    focuses: &[focus(
        0,
        "callback",
        "receives the selected appended-argument shape",
    )],
};
