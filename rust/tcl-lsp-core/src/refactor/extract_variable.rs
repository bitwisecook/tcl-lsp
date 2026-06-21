//! Extract variable — replace a selected expression with a named
//! variable.  Ports `tooling/refactoring/_extract_variable.py`.

use tcl_lexer::LineIndex;

use super::{Refactoring, RefactorEdit};
use crate::code_actions::ActionKind;

/// Whitespace-delimited binary operators that mark an arithmetic /
/// logical expression which must be wrapped in `[expr { … }]` so the
/// resulting `set` stays a valid two-argument call.  Mirrors Python's
/// `_EXPR_OP_RE`.
const EXPR_OPS: &[&str] = &[
    "**", "==", "!=", "<=", ">=", "&&", "||", "eq", "ne", "in", "ni", "+", "-", "*", "/", "%", "<",
    ">",
];

/// `true` when `text` contains a whitespace-delimited binary operator.
///
/// The Python regex requires a single whitespace byte on each side of
/// the operator (`\s OP \s`); this matches that by scanning for ` OP `
/// with single ASCII spaces, which is what the oracle corpus uses.
fn looks_like_expr(text: &str) -> bool {
    EXPR_OPS
        .iter()
        .any(|op| text.contains(&format!(" {op} ")))
}

/// Extract the selection `[start_off, end_off)` into a `set` assignment.
///
/// Ports `extract_variable`.  Returns `None` when the selection is empty
/// or only whitespace.  `start_line` / `start_off` / `end_off` are byte
/// offsets into `source`; `line_index` resolves them to lines for the
/// indentation lookup.
#[must_use]
pub fn extract_variable(
    source: &str,
    start_off: u32,
    end_off: u32,
    var_name: &str,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    if end_off <= start_off {
        return None;
    }
    let selected = source.get(start_off as usize..end_off as usize)?;
    if selected.trim().is_empty() {
        return None;
    }

    let start_line = line_index.line_at(start_off);
    let lines: Vec<&str> = source.split('\n').collect();
    let line_text = lines.get(start_line as usize)?;
    let indent = super::line_indent(line_text);

    // A bare operator expression (`$a * $b`) is not a valid value word
    // for `set` — wrap it in `[expr { … }]`.  A selection that is already
    // a command substitution (`[cmd …]`) or a single word is kept
    // verbatim.
    let stripped = selected.trim();
    let value = if !stripped.starts_with('[') && looks_like_expr(stripped) {
        format!("[expr {{{stripped}}}]")
    } else {
        selected.to_owned()
    };
    let assignment = format!("{indent}set {var_name} {value}\n");

    // The `set` insertion goes at column 0 of the start line; the
    // replacement reference uses the original selection coordinates.
    // `apply` runs bottom-to-top (descending start offset) so the
    // replacement edit (later offset) runs before the line-start
    // insertion, exactly like the Python ordering.
    let line_start = u32::try_from(
        source[..start_off as usize]
            .rfind('\n')
            .map_or(0, |nl| nl + 1),
    )
    .unwrap_or(0);
    let edits = vec![
        RefactorEdit {
            start: line_start,
            end: line_start,
            new_text: assignment,
        },
        RefactorEdit {
            start: start_off,
            end: end_off,
            new_text: format!("${var_name}"),
        },
    ];

    Some(Refactoring {
        title: format!("Extract into variable '${var_name}'"),
        edits,
        kind: ActionKind::RefactorExtract,
        data_group: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source: &str, start: u32, end: u32, name: &str) -> Option<String> {
        let li = LineIndex::new(source);
        extract_variable(source, start, end, name, &li).map(|r| r.apply(source))
    }

    #[test]
    fn extract_command_substitution() {
        let source = "set x [string length $name]";
        // selection of `[string length $name]` (cols 6..27, all ASCII).
        let applied = run(source, 6, 27, "len").expect("result");
        assert!(applied.contains("set len [string length $name]"), "{applied:?}");
        assert!(applied.contains("$len"), "{applied:?}");
    }

    #[test]
    fn title_carries_custom_name() {
        let source = "puts [expr {$a + $b}]";
        let li = LineIndex::new(source);
        let r = extract_variable(source, 5, 20, "total", &li).expect("result");
        assert!(r.title.contains("total"));
    }

    #[test]
    fn empty_selection_returns_none() {
        assert!(run("set x 42", 0, 0, "result").is_none());
    }

    #[test]
    fn whitespace_selection_returns_none() {
        // "set x    42" — cols 5..9 is the run of spaces.
        assert!(run("set x    42", 5, 9, "ws").is_none());
    }

    #[test]
    fn bare_expression_is_wrapped_in_expr() {
        let source = "puts $a + $b";
        let li = LineIndex::new(source);
        let r = extract_variable(source, 5, 12, "sum", &li).expect("result");
        let applied = r.apply(source);
        assert!(applied.contains("set sum [expr {$a + $b}]"), "{applied:?}");
    }
}
