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

//! Enum value sets shared across Tk widget options.  These mirror Tk's own
//! `Tk_GetRelief` / `Tk_GetAnchor` / `Tk_GetJustify` converters, whose accepted
//! spellings are uniform across every core widget — so an option carrying one
//! of these sets can mark it `closed` and let W127 flag a value outside it.

use crate::prelude::*;

/// Standard Tk `relief` values (`Tk_GetRelief`).
pub(crate) const RELIEF: &[ArgValue] = &[
    ArgValue {
        value: "flat",
        detail: "No 3-D border.",
        min_tcl: None,
    },
    ArgValue {
        value: "groove",
        detail: "Grooved (incised) border.",
        min_tcl: None,
    },
    ArgValue {
        value: "raised",
        detail: "Raised 3-D border.",
        min_tcl: None,
    },
    ArgValue {
        value: "ridge",
        detail: "Ridged (embossed) border.",
        min_tcl: None,
    },
    ArgValue {
        value: "solid",
        detail: "Solid one-pixel border.",
        min_tcl: None,
    },
    ArgValue {
        value: "sunken",
        detail: "Sunken 3-D border.",
        min_tcl: None,
    },
];

/// Standard Tk `anchor` positions (`Tk_GetAnchor`).
pub(crate) const ANCHOR: &[ArgValue] = &[
    ArgValue {
        value: "n",
        detail: "North (top center).",
        min_tcl: None,
    },
    ArgValue {
        value: "ne",
        detail: "North-east (top right).",
        min_tcl: None,
    },
    ArgValue {
        value: "e",
        detail: "East (right center).",
        min_tcl: None,
    },
    ArgValue {
        value: "se",
        detail: "South-east (bottom right).",
        min_tcl: None,
    },
    ArgValue {
        value: "s",
        detail: "South (bottom center).",
        min_tcl: None,
    },
    ArgValue {
        value: "sw",
        detail: "South-west (bottom left).",
        min_tcl: None,
    },
    ArgValue {
        value: "w",
        detail: "West (left center).",
        min_tcl: None,
    },
    ArgValue {
        value: "nw",
        detail: "North-west (top left).",
        min_tcl: None,
    },
    ArgValue {
        value: "center",
        detail: "Centered.",
        min_tcl: None,
    },
];

/// Standard Tk `justify` values (`Tk_GetJustify`).
pub(crate) const JUSTIFY: &[ArgValue] = &[
    ArgValue {
        value: "left",
        detail: "Left-justify lines.",
        min_tcl: None,
    },
    ArgValue {
        value: "right",
        detail: "Right-justify lines.",
        min_tcl: None,
    },
    ArgValue {
        value: "center",
        detail: "Center lines.",
        min_tcl: None,
    },
];
