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
//! Shared by every backend — the Python generator calls it through `PyO3`
//! (`_engine.render_markdown`), the Rust generator through `render.rs`, and the
//! in-browser generator through the wasm bindings — so front-matter renders
//! identically no matter which produced the report, and the report stays a
//! single self-contained file (the HTML is inlined, not fetched).
//!
//! Raw embedded HTML is stripped: front-matter is prose, and dropping raw HTML
//! keeps an author (or a pasted snippet) from breaking the single-file document
//! or injecting script into it.

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, html};

/// Whether a link/image destination uses a script-bearing URL scheme that must
/// not survive into the rendered HTML (`javascript:`, `vbscript:`, `data:`).
///
/// Browsers strip ASCII whitespace and control characters and match the scheme
/// case-insensitively, so a payload like `java\tSCRIPT:alert(1)` or
/// ` javascript:…` would still execute — normalise the same way before the
/// prefix test.
fn is_dangerous_url(url: &str) -> bool {
    let normalised: String = url
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .flat_map(char::to_lowercase)
        .collect();
    normalised.starts_with("javascript:")
        || normalised.starts_with("vbscript:")
        || normalised.starts_with("data:")
}

/// Blank out a dangerous destination, leaving a safe one untouched.
fn safe_dest(dest: CowStr<'_>) -> CowStr<'_> {
    if is_dangerous_url(&dest) {
        CowStr::Borrowed("")
    } else {
        dest
    }
}

/// Render `CommonMark` + common GFM extensions (tables, strikethrough, task
/// lists, footnotes, smart punctuation) to an HTML fragment, dropping any raw
/// HTML the source contained and neutralising script-bearing link/image URLs.
#[must_use]
pub fn render_markdown(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(md, opts)
        .filter(|ev| !matches!(ev, Event::Html(_) | Event::InlineHtml(_)))
        .map(|ev| match ev {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => Event::Start(Tag::Link {
                link_type,
                dest_url: safe_dest(dest_url),
                title,
                id,
            }),
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) => Event::Start(Tag::Image {
                link_type,
                dest_url: safe_dest(dest_url),
                title,
                id,
            }),
            other => other,
        });

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

    #[test]
    fn neutralises_script_bearing_link_urls() {
        // A `javascript:` / `data:` / `vbscript:` link
        // destination must not survive into the rendered `href`, even with
        // scheme obfuscation (case, embedded whitespace/control chars).
        for md in [
            "[click](javascript:alert(1))",
            "[click](JavaScript:alert(1))",
            "[click](  javascript:alert(1))",
            "[x](java\tscript:alert(1))",
            "[x](vbscript:msgbox(1))",
            "[x](data:text/html,<script>alert(1)</script>)",
            "![img](javascript:alert(1))",
        ] {
            let html = render_markdown(md);
            assert!(
                !html.contains("javascript:")
                    && !html.contains("vbscript:")
                    && !html.contains("data:text/html"),
                "dangerous scheme survived for {md:?}: {html}",
            );
        }
        // FP-guard: safe schemes and relative links are untouched.
        assert!(
            render_markdown("[x](https://example.com)").contains("href=\"https://example.com\"")
        );
        assert!(render_markdown("[x](/rel/path)").contains("href=\"/rel/path\""));
        assert!(render_markdown("[x](mailto:a@b.com)").contains("href=\"mailto:a@b.com\""));
        assert!(render_markdown("[x](#frag)").contains("href=\"#frag\""));
    }
}
