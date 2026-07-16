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

//! Convert a constant-mapping `switch` to a `dict` lookup.

use tcl_compiler::segmenter::segment_commands_with_offset;
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

use super::{RefactorEdit, Refactoring, find_command_at, token_end_offset};
use crate::code_actions::ActionKind;

/// A parsed single-command arm body.
enum BranchBody {
    /// `set var value` → `(var, raw_value)`.
    Set(String, String),
    /// `return value` → `raw_value`.
    Return(String),
}

/// Parse a single-command switch-branch body via the tokeniser: the
/// value keeps its original source span (quotes / braces intact) so the
/// generated dict preserves the literal.
fn parse_branch_assignment(body_text: &str) -> Option<BranchBody> {
    let commands = segment_commands_with_offset(body_text, 0);
    if commands.len() != 1 || commands[0].texts.is_empty() {
        return None;
    }
    let cmd = &commands[0];
    let raw = |index: usize| -> String {
        let tok = cmd.argv[index];
        body_text[tok.span.start() as usize..token_end_offset(body_text, tok) as usize].to_owned()
    };
    if cmd.texts[0] == "set" && cmd.texts.len() == 3 {
        return Some(BranchBody::Set(cmd.texts[1].clone(), raw(2)));
    }
    if cmd.texts[0] == "return" && cmd.texts.len() == 2 {
        return Some(BranchBody::Return(raw(1)));
    }
    None
}

/// Convert a `switch` where every arm sets the same variable / returns a
/// constant into a `dict` lookup at byte offset `cursor`.
#[must_use]
pub fn switch_to_dict(
    source: &str,
    cursor: u32,
    registry: &CommandRegistry,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    let cmd = find_command_at(source, cursor, Some("switch"), registry)?;
    let texts = &cmd.texts;
    if texts.len() < 3 {
        return None;
    }

    // Only `-exact` mode is convertible; the helper yields the subject and
    // the pattern/body pairs (single braced list or separate words).
    let (subject, pairs) = super::parse_exact_switch(texts)?;
    if pairs.is_empty() {
        return None;
    }

    let arms = parse_arms(&pairs)?;
    if arms.dict_entries.len() < 2 {
        return None;
    }

    let indent = source
        .split('\n')
        .nth(line_index.line_at(cmd.span.start()) as usize)
        .map_or("", super::line_indent);
    let replacement = build_dict_replacement(&arms, &subject, indent);

    let (start, end) = super::command_span_offsets(source, &cmd);
    let title = if arms.use_return {
        "Convert to dict lookup".to_owned()
    } else {
        format!("Convert to dict lookup on '{}'", arms.target_var)
    };
    Some(Refactoring {
        title,
        edits: vec![RefactorEdit {
            start,
            end,
            new_text: replacement,
        }],
        kind: ActionKind::RefactorRewrite,
        data_group: None,
    })
}

/// The arms of a convertible switch, all of one shape.
struct ParsedArms {
    target_var: String,
    use_return: bool,
    dict_entries: Vec<(String, String)>,
    default_value: Option<String>,
}

/// Parse every arm, requiring a single uniform `set VAR …` / `return …`
/// shape.  Returns `None` on a fallthrough marker, a body that is neither
/// form, or a mixed shape.
fn parse_arms(pairs: &[(String, String)]) -> Option<ParsedArms> {
    let mut target_var: Option<String> = None;
    let mut use_return = false;
    let mut dict_entries: Vec<(String, String)> = Vec::new();
    let mut default_value: Option<String> = None;

    for (pattern, body) in pairs {
        let mut body_text = body.trim();
        if body_text.starts_with('{') && body_text.ends_with('}') && body_text.len() >= 2 {
            body_text = body_text[1..body_text.len() - 1].trim();
        }
        if body_text == "-" {
            return None; // fallthrough marker
        }
        let value = match parse_branch_assignment(body_text)? {
            BranchBody::Set(var, value) => {
                match &target_var {
                    None => {
                        target_var = Some(var);
                        use_return = false;
                    }
                    Some(v) if *v != var => return None,
                    _ => {}
                }
                if use_return {
                    return None;
                }
                value
            }
            BranchBody::Return(value) => {
                match &target_var {
                    None => {
                        target_var = Some("__return__".to_owned());
                        use_return = true;
                    }
                    Some(_) if !use_return => return None,
                    _ => {}
                }
                value
            }
        };
        if pattern == "default" {
            default_value = Some(value);
        } else {
            dict_entries.push((pattern.clone(), value));
        }
    }

    Some(ParsedArms {
        target_var: target_var?,
        use_return,
        dict_entries,
        default_value,
    })
}

/// Build the `dict create` + lookup replacement from the parsed arms.
fn build_dict_replacement(arms: &ParsedArms, subject: &str, indent: &str) -> String {
    let dict_items = arms
        .dict_entries
        .iter()
        .map(|(k, v)| format!("{k} {v}"))
        .collect::<Vec<_>>()
        .join(" ");
    let dict_name = if arms.use_return {
        "result_map".to_owned()
    } else {
        format!("{}_map", arms.target_var)
    };
    let target_var = &arms.target_var;

    let mut parts: Vec<String> = vec![format!("set {dict_name} [dict create {dict_items}]")];
    if let Some(default) = &arms.default_value {
        parts.push(format!(
            "{indent}if {{[dict exists ${dict_name} {subject}]}} {{"
        ));
        if arms.use_return {
            parts.push(format!(
                "{indent}    return [dict get ${dict_name} {subject}]"
            ));
            parts.push(format!("{indent}}} else {{"));
            parts.push(format!("{indent}    return {default}"));
        } else {
            parts.push(format!(
                "{indent}    set {target_var} [dict get ${dict_name} {subject}]"
            ));
            parts.push(format!("{indent}}} else {{"));
            parts.push(format!("{indent}    set {target_var} {default}"));
        }
        parts.push(format!("{indent}}}"));
    } else if arms.use_return {
        parts.push(format!("{indent}return [dict get ${dict_name} {subject}]"));
    } else {
        parts.push(format!(
            "{indent}set {target_var} [dict get ${dict_name} {subject}]"
        ));
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source: &str, cursor: u32) -> Option<Refactoring> {
        let reg = super::super::test_registry();
        let li = LineIndex::new(source);
        switch_to_dict(source, cursor, &reg, &li)
    }

    #[test]
    fn set_pattern() {
        let source = "switch -exact -- $method {\n    GET { set handler handle_get }\n    POST { set handler handle_post }\n    PUT { set handler handle_put }\n}";
        let r = run(source, 0).expect("result");
        assert!(r.title.to_lowercase().contains("dict"));
        let applied = r.apply(source);
        // The segmenter canonicalises the `$method` subject to
        // `${method}`.
        assert_eq!(
            applied,
            "set handler_map [dict create GET handle_get POST handle_post PUT handle_put]\nset handler [dict get $handler_map ${method}]"
        );
        assert!(applied.contains("dict create"), "{applied:?}");
        assert!(applied.contains("dict get"), "{applied:?}");
    }

    #[test]
    fn return_pattern() {
        let source = "switch -exact -- $code {\n    200 { return \"OK\" }\n    404 { return \"Not Found\" }\n    500 { return \"Server Error\" }\n}";
        let applied = run(source, 0).expect("result").apply(source);
        assert!(applied.contains("dict create"), "{applied:?}");
    }

    #[test]
    fn glob_mode_returns_none() {
        let source = "switch -glob -- $path {\n    /api/* { set handler api }\n    /web/* { set handler web }\n}";
        assert!(run(source, 0).is_none());
    }

    #[test]
    fn too_few_arms_returns_none() {
        assert!(run("switch -exact -- $x {\n    a { set y 1 }\n}", 0).is_none());
    }

    #[test]
    fn mixed_bodies_returns_none() {
        let source = "switch -exact -- $x {\n    a { set y 1 }\n    b { puts \"hello\" }\n}";
        assert!(run(source, 0).is_none());
    }

    #[test]
    fn rewrite_covers_entire_switch_command() {
        let source = "switch -exact -- $m {\n    a { set out 1 }\n    b { set out 2 }\n    c { set out 3 }\n}";
        let applied = run(source, 0).expect("result").apply(source);
        let applied = applied.trim();
        assert!(applied.ends_with(']'), "{applied:?}");
        assert!(!applied.ends_with("]}"), "{applied:?}");
    }

    #[test]
    fn indented_switch_does_not_double_indent_first_line() {
        let source = "    switch -exact -- $method {\n        GET { set handler get_h }\n        POST { set handler post_h }\n        PUT { set handler put_h }\n    }";
        // Cursor at col 4 (offset 4) — on the `switch` keyword.
        let applied = run(source, 4).expect("result").apply(source);
        let first_line = applied.split('\n').next().unwrap();
        assert!(first_line.starts_with("    set "), "{first_line:?}");
        assert!(!first_line.starts_with("        set "), "{first_line:?}");
    }

    #[test]
    fn inside_when_body() {
        let source = "when HTTP_REQUEST {\n    switch -exact -- $method {\n        GET { set handler get_h }\n        POST { set handler post_h }\n        PUT { set handler put_h }\n    }\n}";
        // `when`'s body role lives in the iRules dialect.
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(tcl_dialect::DialectSet::IRULES);
        let li = LineIndex::new(source);
        let cursor = u32::try_from(source.find("switch").unwrap()).unwrap();
        let r = switch_to_dict(source, cursor, &reg, &li).expect("nested result");
        assert!(r.title.to_lowercase().contains("dict"));
    }
}
