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

//! Highlighting through `highlight_ansi` and `highlight_html`, the
//! renderers the CLI writes. A command word renders bold blue
//! (`ESC[1;34m`), and stripping the escapes gives back the source bytes.

use tcl_cli_support::environment::profile_for_dialect;
use tcl_cli_support::{highlight_ansi, highlight_html};

const COMMAND: &str = "\x1b[1;34m";
const RESET: &str = "\x1b[0m";

/// The rendered text without its SGR escapes.
fn plain(rendered: &str) -> String {
    let mut out = String::with_capacity(rendered.len());
    let mut rest = rendered;
    while let Some(start) = rest.find('\x1b') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let end = tail.find('m').expect("an SGR escape ends in `m`");
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

fn blue(rendered: &str, command: &str) -> bool {
    rendered.contains(&format!("{COMMAND}{command}{RESET}"))
}

#[test]
fn original_highlight_keeps_data_inert_and_single_line_bodies_visible() {
    // naming.cli.original-source-highlight-geometry
    // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
    crate::install_surfaces();
    let profile = profile_for_dialect("tcl8.6");
    for (source, inner, expected) in [
        ("set value {\nnotACommand\n}", "notACommand", false),
        ("set value {notACommand; stillData}", "notACommand", false),
        ("if {1} {puts single}", "puts", true),
        ("if {1} {puts café🙂}", "puts", true),
        ("set data café🙂; if {1} {  puts offset  }", "puts", true),
        ("if {1} {if {1} {puts nested}}", "puts", true),
    ] {
        let rendered = highlight_ansi(source, profile);
        assert_eq!(blue(&rendered, inner), expected, "{rendered}");
        assert_eq!(plain(&rendered), source);
    }
}

#[test]
fn original_highlight_html_shares_script_regions_and_source_escaping() {
    // naming.cli.original-source-highlight-geometry
    // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
    crate::install_surfaces();
    let profile = profile_for_dialect("tcl8.6");
    let data = highlight_html("set value {\nnotACommand\n}", profile);
    assert!(!data.contains("font-weight:600;\">notACommand</span>"));
    let body = highlight_html("if {1} {puts <café🙂>&}", profile);
    assert!(body.contains("font-weight:600;\">puts</span>"), "{body}");
    assert!(body.contains("&lt;café🙂&gt;&amp;"), "{body}");
    assert_eq!(highlight_html("", profile), "<pre></pre>\n");
}

#[test]
fn original_highlight_keeps_selected_alias_prefix_and_shadow_roles() {
    // naming.cli.original-source-highlight-geometry
    // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
    crate::install_surfaces();
    let profile = profile_for_dialect("tcl8.6");
    for (source, expected) in [
        ("rename if condition; condition {1} {puts moved}", true),
        (
            "interp alias {} condition {} if 1; condition {puts prefix}",
            true,
        ),
        ("proc if {a b} {}; if {1} {puts data}", false),
        ("unknown {puts data}", false),
    ] {
        let rendered = highlight_ansi(source, profile);
        assert_eq!(blue(&rendered, "puts"), expected, "{rendered}");
        assert_eq!(plain(&rendered), source);
    }
}

#[test]
fn original_highlight_uses_actual_c_jim_and_irules_lexical_axes() {
    // naming.cli.original-source-highlight-geometry
    // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
    crate::install_surfaces();
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = profile_for_dialect(name);
        let source = "if {1} {puts dialect}";
        let rendered = highlight_ansi(source, profile);
        assert!(blue(&rendered, "puts"), "{name}: {rendered}");
        assert_eq!(plain(&rendered), source);
    }
    let profile = profile_for_dialect("f5-irules");
    let source = "when HTTP_REQUEST{puts café🙂}";
    let rendered = highlight_ansi(source, profile);
    assert!(blue(&rendered, "puts"), "{rendered}");
    assert_eq!(plain(&rendered), source);
}
