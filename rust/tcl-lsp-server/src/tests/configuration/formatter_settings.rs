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

//! The formatter configuration built from settings.

use super::*;

#[test]
fn formatter_config_honours_request_indent() {
    let mut opts = tower_lsp_server::ls_types::FormattingOptions {
        tab_size: 2,
        insert_spaces: true,
        ..Default::default()
    };
    let cfg = formatter_config_from(
        &serde_json::Value::Null,
        &opts,
        tcl_lsp_core::profile_for_dialect("tcl"),
    );
    assert_eq!(cfg.indent_size, 2);
    assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Spaces);
    // A degenerate zero tabSize is ignored (editors always send >= 1), so
    // the configured / default indent size stands (4 here).
    opts.tab_size = 0;
    assert_eq!(
        formatter_config_from(
            &serde_json::Value::Null,
            &opts,
            tcl_lsp_core::profile_for_dialect("tcl")
        )
        .indent_size,
        4
    );
    // insertSpaces=false selects tab indentation.
    opts.tab_size = 4;
    opts.insert_spaces = false;
    let cfg = formatter_config_from(
        &serde_json::Value::Null,
        &opts,
        tcl_lsp_core::profile_for_dialect("tcl"),
    );
    assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Tabs);
}

#[test]
fn formatter_config_carries_the_documents_resolved_dialect() {
    // The document's detected dialect (`DocumentState::dialect`) is the
    // formatter's one dialect fact: the lexer preset, the
    // rewrite-candidate mask, and the forward range all follow from the
    // profile it resolves.
    let opts = tower_lsp_server::ls_types::FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        ..Default::default()
    };
    for spelling in ["f5-irules", "irules"] {
        let cfg = formatter_config_from(
            &serde_json::Value::Null,
            &opts,
            tcl_lsp_core::profile_for_dialect(spelling),
        );
        assert!(cfg.profile.is_irules(), "{spelling}");
        // The `}{` ghost separator TMM accepts, which a Tcl 9 lexer does
        // not parse as two words.
        assert!(cfg.lexer_config().irules_brace_separator, "{spelling}");
        // A vendor dialect names no core release: no forward range.
        assert!(cfg.target_range().is_empty(), "{spelling}");
    }
    let cfg = formatter_config_from(
        &serde_json::Value::Null,
        &opts,
        tcl_lsp_core::profile_for_dialect("tcl8.6"),
    );
    assert!(!cfg.lexer_config().irules_brace_separator);
    assert_eq!(
        cfg.target_range(),
        tcl_registry::version_range::forward_range("tcl8.6")
    );
}

#[test]
fn formatter_config_from_consumes_formatting_settings() {
    // `tclLsp.formatting.*` settings flow into the FormatterConfig, while
    // the request's LSP indentation options override indent size/style.
    let formatting = serde_json::json!({
        "maxLineLength": 100,
        "goalLineLength": 90,
        "indentSize": 8,
        "indentStyle": "tabs",
        "spaceBetweenBraces": false,
        "lineEnding": "crlf",
        "blankLinesBetweenProcs": 2,
    });
    let opts = tower_lsp_server::ls_types::FormattingOptions {
        tab_size: 2,
        insert_spaces: true,
        ..Default::default()
    };
    let cfg = formatter_config_from(&formatting, &opts, tcl_lsp_core::profile_for_dialect("tcl"));
    assert_eq!(cfg.max_line_length, 100);
    assert_eq!(cfg.goal_line_length, 90);
    assert!(!cfg.space_between_braces);
    assert_eq!(cfg.line_ending, "\r\n");
    assert_eq!(cfg.blank_lines_between_procs, 2);
    // LSP options win for indentation (tabSize=2, insertSpaces=true).
    assert_eq!(cfg.indent_size, 2);
    assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Spaces);
    // A null formatting object falls back to defaults + LSP options.
    let dflt = formatter_config_from(
        &serde_json::Value::Null,
        &opts,
        tcl_lsp_core::profile_for_dialect("tcl"),
    );
    assert_eq!(dflt.max_line_length, 120);
}

#[test]
fn formatter_config_consumes_previously_dropped_settings() {
    // These four `tclLsp.formatting.*` settings were shipped
    // by every editor but never mapped, so toggling them did nothing. They
    // must now flow into the FormatterConfig.
    let opts = tower_lsp_server::ls_types::FormattingOptions::default();
    let cfg = formatter_config_from(
        &serde_json::json!({
            "minBodyCommandsForExpansion": 3,
            "replaceSemicolonsWithNewlines": false,
            "enforceBracedExpr": true,
            "alignCommentsToCode": false,
        }),
        &opts,
        tcl_lsp_core::profile_for_dialect("tcl"),
    );
    assert_eq!(cfg.min_body_commands_for_expansion, 3);
    assert!(!cfg.replace_semicolons_with_newlines);
    assert!(cfg.enforce_braced_expr);
    assert!(!cfg.align_comments_to_code);
}

#[test]
fn formatter_config_round_trips_docstring_settings() {
    // The docstring knobs are carried for config-consumer parity on
    // `FormatterConfig` (`docstringStyle`'s real consumer is the
    // `code_action` handler via `resolved_docstring_style`, tested
    // below); a settings object still flows them into this config too.
    let opts = tower_lsp_server::ls_types::FormattingOptions::default();
    let cfg = formatter_config_from(
        &serde_json::json!({
            "docstringStyle": "preceding",
            "docstringTagStyle": "plain",
            "docstringDecoration": true,
            "docstringDecorationChar": "=",
            "docstringDecorationWidth": 80,
        }),
        &opts,
        tcl_lsp_core::profile_for_dialect("tcl"),
    );
    assert_eq!(
        cfg.docstring_style,
        core_formatting::DocstringStyle::Preceding
    );
    assert_eq!(
        cfg.docstring_tag_style,
        core_formatting::DocstringTagStyle::Plain
    );
    assert!(cfg.docstring_decoration);
    assert_eq!(cfg.docstring_decoration_char, '=');
    assert_eq!(cfg.docstring_decoration_width, 80);
    // Defaults are style none, tag doxygen, char '.'.
    let dflt = core_formatting::FormatterConfig::default();
    assert_eq!(dflt.docstring_style, core_formatting::DocstringStyle::None);
    assert_eq!(
        dflt.docstring_tag_style,
        core_formatting::DocstringTagStyle::Doxygen
    );
    assert_eq!(dflt.docstring_decoration_char, '.');
    assert_eq!(dflt.docstring_decoration_width, 70);
}
