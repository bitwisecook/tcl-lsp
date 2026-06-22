//! Convert an `if`/`elseif` equality chain to a `switch` statement.
//! Ports `tooling/refactoring/_if_to_switch.py`.

use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

use super::{Refactoring, RefactorEdit, find_command_at, reindent_body};
use crate::code_actions::ActionKind;

/// Parsed equality condition: `(var_name, value, negated)`.
struct EqTest {
    var: String,
    value: String,
    negated: bool,
}

/// Parse an equality condition `$var eq "value"` / `"value" eq $var`
/// (and the `==` / `ne` / `!=` operators).  Ports `_parse_eq_test`.
fn parse_eq_test(condition: &str) -> Option<EqTest> {
    let mut cond = condition.trim();

    // Strip outer braces: `{ $x eq "foo" }`.
    if cond.starts_with('{') && cond.ends_with('}') && cond.len() >= 2 {
        cond = cond[1..cond.len() - 1].trim();
    }

    // Negation: `!($x eq "foo")` or `!{$x eq "foo"}`.
    let mut negated = false;
    if let Some(rest) = cond.strip_prefix('!') {
        let inner = rest.trim();
        if inner.len() >= 2
            && ((inner.starts_with('(') && inner.ends_with(')'))
                || (inner.starts_with('{') && inner.ends_with('}')))
        {
            cond = inner[1..inner.len() - 1].trim();
            negated = true;
        }
    }

    if let Some((var, op, value)) = split_var_op_value(cond) {
        let is_ne = op == "ne" || op == "!=";
        return Some(EqTest {
            var,
            value: value.trim().to_owned(),
            negated: negated ^ is_ne,
        });
    }
    if let Some((value, op, var)) = split_value_op_var(cond) {
        let is_ne = op == "ne" || op == "!=";
        return Some(EqTest {
            var,
            value: value.trim().to_owned(),
            negated: negated ^ is_ne,
        });
    }
    None
}

/// Recognised equality operators, longest-first so `==` wins over a bare
/// fragment.
const OPS: &[&str] = &["==", "!=", "eq", "ne"];

/// Split `$var OP value` / `"$var" OP value` → `(var, op, value)`.
fn split_var_op_value(cond: &str) -> Option<(String, String, String)> {
    for op in OPS {
        let needle = format!(" {op} ");
        if let Some(pos) = cond.find(&needle) {
            let lhs = cond[..pos].trim();
            let value = cond[pos + needle.len()..].to_owned();
            if let Some(var) = parse_var_word(lhs) {
                return Some((var, (*op).to_owned(), value));
            }
        }
    }
    None
}

/// Split `value OP $var` / `value OP "$var"` → `(value, op, var)`.
fn split_value_op_var(cond: &str) -> Option<(String, String, String)> {
    for op in OPS {
        let needle = format!(" {op} ");
        // Use the *last* occurrence so a value containing the operator
        // text doesn't mis-split (the Python regex anchors the var on
        // the right with `$`).
        if let Some(pos) = cond.rfind(&needle) {
            let value = cond[..pos].trim().to_owned();
            let rhs = cond[pos + needle.len()..].trim();
            if let Some(var) = parse_var_word(rhs) {
                return Some((value, (*op).to_owned(), var));
            }
        }
    }
    None
}

/// Parse a `$var` / `${var}` / `"$var"` / `"${var}"` word to its bare
/// variable name (`[A-Za-z0-9_]+`), or `None`.  Mirrors the regex
/// `\$\{?(\w+)\}?` (optionally wrapped in double quotes).
fn parse_var_word(word: &str) -> Option<String> {
    let mut w = word.trim();
    if w.len() >= 2 && w.starts_with('"') && w.ends_with('"') {
        w = &w[1..w.len() - 1];
    }
    let w = w.strip_prefix('$')?;
    let w = w.strip_prefix('{').unwrap_or(w);
    let w = w.strip_suffix('}').unwrap_or(w);
    if !w.is_empty() && w.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Some(w.to_owned())
    } else {
        None
    }
}

/// Remove surrounding double quotes if present.
fn strip_quotes(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"' {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Render a switch pattern, preserving whitespace literals by bracing.
/// Ports `_render_switch_pattern`.
fn render_switch_pattern(value: &str) -> String {
    if value.is_empty() {
        return "{}".to_owned();
    }
    if value.chars().any(char::is_whitespace) {
        format!("{{{}}}", value.replace('}', "\\}"))
    } else {
        value.to_owned()
    }
}

/// Convert an `if`/`elseif` chain at byte offset `cursor` to a `switch`.
/// Ports `if_to_switch`.
#[must_use]
pub fn if_to_switch(
    source: &str,
    cursor: u32,
    registry: &CommandRegistry,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    let cmd = find_command_at(source, cursor, Some("if"), registry)?;
    let texts = &cmd.texts;
    if texts.len() < 3 {
        return None;
    }

    // Parse the if/elseif chain: `if cond body ?elseif cond body?... ?else body?`.
    let mut branches: Vec<(String, String)> = Vec::new(); // (value, body)
    let mut else_body: Option<String> = None;
    let mut target_var: Option<String> = None;

    let mut i = 1;
    while i < texts.len() {
        let word = &texts[i];
        if word == "elseif" || word == "then" {
            i += 1;
            continue;
        }
        if word == "else" {
            if i + 1 < texts.len() {
                else_body = Some(texts[i + 1].clone());
            }
            break;
        }

        // This should be a condition.
        let condition = word.clone();
        if i + 1 >= texts.len() {
            return None;
        }
        let mut body = texts[i + 1].clone();
        i += 2;

        // Skip an optional `then` after the condition.
        if i < texts.len() && texts[i] == "then" {
            i += 1;
            if i < texts.len() {
                body.clone_from(&texts[i]);
                i += 1;
            }
        }

        let parsed = parse_eq_test(&condition)?;
        match &target_var {
            None => target_var = Some(parsed.var.clone()),
            Some(v) if *v != parsed.var => return None,
            _ => {}
        }
        // Negated (ne / !=) conditions make switch conversion awkward.
        if parsed.negated {
            return None;
        }
        branches.push((strip_quotes(&parsed.value).to_owned(), body));
    }

    let target_var = target_var?;
    if branches.len() < 2 {
        return None;
    }

    // Build the switch statement.
    let lines: Vec<&str> = source.split('\n').collect();
    let cmd_line = line_index.line_at(cmd.span.start());
    let indent = lines
        .get(cmd_line as usize)
        .map_or("", |l| super::line_indent(l));
    let inner = format!("{indent}    ");

    let mut parts: Vec<String> = vec![format!("switch -exact -- ${target_var} {{")];
    for (value, body) in &branches {
        let body_trimmed = reindent_body(body, &format!("{inner}    "));
        parts.push(format!("{inner}{} {{", render_switch_pattern(value)));
        parts.push(body_trimmed);
        parts.push(format!("{inner}}}"));
    }
    if let Some(eb) = &else_body {
        let body_trimmed = reindent_body(eb, &format!("{inner}    "));
        parts.push(format!("{inner}default {{"));
        parts.push(body_trimmed);
        parts.push(format!("{inner}}}"));
    }
    parts.push(format!("{indent}}}"));
    let replacement = parts.join("\n");

    let (start, end) = super::command_span_offsets(source, &cmd);
    Some(Refactoring {
        title: format!("Convert to switch on ${target_var}"),
        edits: vec![RefactorEdit {
            start,
            end,
            new_text: replacement,
        }],
        kind: ActionKind::RefactorRewrite,
        data_group: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source: &str, cursor: u32) -> Option<Refactoring> {
        let reg = super::super::test_registry();
        let li = LineIndex::new(source);
        if_to_switch(source, cursor, &reg, &li)
    }

    #[test]
    fn simple_eq_chain() {
        let source = "if {$x eq \"a\"} {\n    puts \"alpha\"\n} elseif {$x eq \"b\"} {\n    puts \"beta\"\n} elseif {$x eq \"c\"} {\n    puts \"gamma\"\n}";
        let r = run(source, 0).expect("result");
        assert!(r.title.to_lowercase().contains("switch"));
        let applied = r.apply(source);
        assert!(applied.contains("switch -exact -- $x"), "{applied:?}");
    }

    #[test]
    fn with_else_clause() {
        let source = "if {$cmd eq \"GET\"} {\n    handle_get\n} elseif {$cmd eq \"POST\"} {\n    handle_post\n} else {\n    handle_other\n}";
        let r = run(source, 0).expect("result");
        assert!(r.apply(source).contains("default"));
    }

    #[test]
    fn different_vars_returns_none() {
        let source = "if {$x eq \"a\"} {\n    puts \"x\"\n} elseif {$y eq \"b\"} {\n    puts \"y\"\n}";
        assert!(run(source, 0).is_none());
    }

    #[test]
    fn single_branch_returns_none() {
        assert!(run("if {$x eq \"a\"} { puts \"alpha\" }", 0).is_none());
    }

    #[test]
    fn ne_operator_returns_none() {
        let source = "if {$x ne \"a\"} {\n    puts \"not a\"\n} elseif {$x ne \"b\"} {\n    puts \"not b\"\n}";
        assert!(run(source, 0).is_none());
    }

    #[test]
    fn rewrite_covers_entire_if_command() {
        let source = "if {$x eq \"a\"} {\n    puts 1\n} elseif {$x eq \"b\"} {\n    puts 2\n}";
        let applied = run(source, 0).expect("result").apply(source);
        // Byte-for-byte parity with the Python oracle.
        assert_eq!(
            applied,
            "switch -exact -- $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}"
        );
        let trimmed = applied.trim();
        assert!(trimmed.ends_with('}'));
        assert!(!trimmed.ends_with("}}"), "{trimmed:?}");
    }

    #[test]
    fn values_with_spaces_are_braced() {
        let source = "if {$x eq \"a b\"} {\n    puts one\n} elseif {$x eq \"c d\"} {\n    puts two\n}";
        let applied = run(source, 0).expect("result").apply(source);
        assert!(applied.contains("{a b}"), "{applied:?}");
        assert!(applied.contains("{c d}"), "{applied:?}");
    }

    #[test]
    fn inside_proc_body() {
        let source = "proc handler {method} {\n    if {$method eq \"GET\"} {\n        handle_get\n    } elseif {$method eq \"POST\"} {\n        handle_post\n    } elseif {$method eq \"PUT\"} {\n        handle_put\n    }\n}";
        // Cursor at line 1, col 4 — inside the `if`.
        let cursor = source.find("if {").unwrap() as u32;
        let r = run(source, cursor).expect("nested result");
        assert!(r.title.to_lowercase().contains("switch"));
    }
}
