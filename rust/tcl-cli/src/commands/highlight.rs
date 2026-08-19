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

//! The `highlight` verb — emit ANSI- or HTML-highlighted source.
//!
//! Note the tab
//! handling differs from the transform verbs: here highlighting runs *first*
//! and tab expansion *after* (so tab expansion sees the escape codes), which
//! is the order for this verb specifically.

use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, expand_tabs, highlight_ansi,
    highlight_html, read_input_documents, resolve_use_colour, write_text_output,
};

use crate::cli::{ColourArgs, InputArgs};

/// Default tab-expansion width on stdout (the CLI default).
const DEFAULT_TAB_WIDTH: usize = 4;

/// `tcl highlight` — emit syntax-highlighted source (ANSI or HTML).
pub fn run_highlight(input: &InputArgs, format: &str, colour: &ColourArgs) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);
    let target = OutputTarget::from_arg(input.output.as_deref());

    let mut out = if format == "html" {
        highlight_html(&source, dialect)
    } else if resolve_use_colour(colour.colour, colour.no_colour, &target) {
        highlight_ansi(&source, dialect)
    } else {
        source
    };

    if target.is_stdout() && DEFAULT_TAB_WIDTH > 0 {
        out = expand_tabs(&out, DEFAULT_TAB_WIDTH);
    }
    write_text_output(&target, &out)?;
    Ok(0)
}
