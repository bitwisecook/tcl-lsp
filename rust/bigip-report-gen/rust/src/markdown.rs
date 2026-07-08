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

//! The single Markdown → HTML renderer for user-supplied report front-matter.
//!
//! Shared by every backend — the Python generator calls it through PyO3
//! (`_engine.render_markdown`), the Rust generator through `render.rs`, and the
//! in-browser generator through the wasm bindings — so front-matter renders
//! identically no matter which produced the report, and the report stays a
//! single self-contained file (the HTML is inlined, not fetched).
//!
//! Raw embedded HTML is stripped: front-matter is prose, and dropping raw HTML
//! keeps an author (or a pasted snippet) from breaking the single-file document
//! or injecting script into it.

use pulldown_cmark::{Event, Options, Parser, html};

/// Render CommonMark + common GFM extensions (tables, strikethrough, task
/// lists, footnotes, smart punctuation) to an HTML fragment, dropping any raw
/// HTML the source contained.
pub fn render_markdown(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser =
        Parser::new_ext(md, opts).filter(|ev| !matches!(ev, Event::Html(_) | Event::InlineHtml(_)));

    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

#[cfg(test)]
mod tests {
    use super::render_markdown;

    #[test]
    fn renders_basic_markdown() {
        let html = render_markdown("# Title\n\nSome **bold** and a [link](https://example.com).");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("href=\"https://example.com\""));
    }

    #[test]
    fn strips_raw_html() {
        let html = render_markdown("ok <script>alert(1)</script> done");
        assert!(
            !html.contains("<script>"),
            "raw HTML must be stripped: {html}"
        );
    }

    #[test]
    fn renders_tables() {
        let html = render_markdown("| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(html.contains("<table>"));
    }
}
