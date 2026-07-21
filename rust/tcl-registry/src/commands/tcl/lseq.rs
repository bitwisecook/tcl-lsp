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

//! `lseq` — generate a list of numeric values in a range (Tcl 9.0).
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lseq ?start? ?op? end ?by step? ?count n?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lseq",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::new(1, 5),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Generate a list of numeric values in a range.",
            synopsis: &[
                "lseq n",
                "lseq start end ?step?",
                "lseq start to end",
                "lseq start 'count' count",
                "lseq start 'by' step 'count' count",
            ],
            snippet: "Returns a list of numbers from start through end (inclusive) with optional step.  One-arg form yields 0..n-1.  Float and double values are supported.",
            source: "Tcl man page lseq.n (Tcl 9.0)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
