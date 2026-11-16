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

use super::*;
fn condition_to_ip_prefix_set(node: &ExprNode) -> Option<String> {
    let ExprNode::Command { text, .. } = node else {
        return None;
    };
    let parts = parse_command_parts(text);
    if parts.len() == 5
        && parts[0] == "class"
        && parts[1] == "match"
        && original_command_words(text)
            .and_then(|words| scalar_getter_operand(words.get(2)?))
            .as_deref()
            == Some("IP::client_addr")
    {
        return parts.last().cloned();
    }
    None
}

fn condition_to_class_match_method(node: &ExprNode) -> Option<XCMethodMatch> {
    let ExprNode::Command { text, .. } = node else {
        return None;
    };
    let parts = parse_command_parts(text);
    if parts.len() == 5
        && parts[0] == "class"
        && parts[1] == "match"
        && original_command_words(text)
            .and_then(|words| scalar_getter_operand(words.get(2)?))
            .as_deref()
            == Some("HTTP::method")
    {
        // Data-group name is last part — cannot resolve statically. Return a
        // placeholder match indicating method matching.
        return Some(XCMethodMatch {
            methods: Vec::new(),
            invert: false,
        });
    }
    None
}

#[test]
fn scalar_getter_advice_matches_original_command_word_geometry() {
    for text in [
        "[HTTP::path]",
        "[{HTTP::path}]",
        "[\"HTTP::path\"]",
        r"[HTTP\x3a\x3apath]",
        "[string tolower [HTTP::path]]",
        "[string {tolower} [HTTP::path]]",
    ] {
        assert_eq!(
            is_http_getter(text).as_deref(),
            Some("HTTP::path"),
            "{text}"
        );
    }
    for text in [
        "[string tolower {[HTTP::path]}]",
        "[HTTP::path extra]",
        "[HTTP::path; HTTP::method]",
        "[HTTP::path]suffix",
        "[$getter]",
        "[HTTP::path",
        "[{*}HTTP::path]",
        "[string tolower [HTTP::path] extra]",
    ] {
        assert_eq!(is_http_getter(text), None, "{text}");
    }
}

#[test]
fn class_match_advice_reads_the_original_operand_instead_of_substrings() {
    let node = |text: &str| parse_expr(text, Some("f5-irules"));
    assert_eq!(
        condition_to_ip_prefix_set(&node("[class match [IP::client_addr] equals {allow set}]")),
        Some("allow set".into())
    );
    assert!(
        condition_to_ip_prefix_set(&node("[class match literal equals IP::client_addr]")).is_none()
    );
    assert!(
        condition_to_class_match_method(&node("[class match [HTTP::method] equals methods]"))
            .is_some()
    );
    assert!(
        condition_to_class_match_method(&node("[class match literal equals HTTP::method]"))
            .is_none()
    );
    assert!(
        condition_to_ip_prefix_set(&node("[class match {[IP::client_addr]} equals allow]"))
            .is_none()
    );
    assert!(
        condition_to_class_match_method(&node("[class match {[HTTP::method]} equals methods]"))
            .is_none()
    );
}
