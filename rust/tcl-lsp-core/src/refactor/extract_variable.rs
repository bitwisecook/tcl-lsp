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

//! Extract variable — replace a selected expression with a named
//! variable.

use tcl_lexer::LineIndex;

use super::{RefactorEdit, Refactoring};
use crate::code_actions::ActionKind;

/// Whitespace-delimited binary operators that mark an arithmetic /
/// logical expression which must be wrapped in `[expr { … }]` so the
/// resulting `set` stays a valid two-argument call.
///
/// Every [`BinOp`](tcl_syntax::expr::ast::BinOp) variant is, by
/// construction, a genuine infix binary operator — derived from
/// `tcl_syntax::expr::operators::ALL_BIN_OPS` (issue #983's unification)
/// rather than a hand-typed 17-entry list that used to miss the bitwise/
/// shift symbols (`<<`/`>>`/`&`/`|`/`^`), the TIP 461 string-ordering words
/// (`lt`/`le`/`gt`/`ge`), and every iRules word operator (`contains`/
/// `starts_with`/…). That wasn't just a missed suggestion: selecting
/// `$a << 2` and extracting it produced `set myvar $a << 2` — a 4-argument
/// `set` call, which is a Tcl runtime error (`set` takes 1 or 2 args), not
/// merely a semantic difference.
fn expr_op_spellings() -> &'static [&'static str] {
    static OPS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    OPS.get_or_init(|| {
        tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .map(|op| op.spec().spelling)
            .collect()
    })
}

/// `true` when `text` contains a whitespace-delimited binary operator.
///
/// A whitespace-delimited operator has a single whitespace byte on
/// each side (`\s OP \s`); this scans for ` OP ` with single ASCII
/// spaces.
fn looks_like_expr(text: &str) -> bool {
    expr_op_spellings()
        .iter()
        .any(|op| text.contains(&format!(" {op} ")))
}

/// Extract the selection `[start_off, end_off)` into a `set` assignment.
///
/// Returns `None` when the selection is empty
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
    // insertion.
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
        assert!(
            applied.contains("set len [string length $name]"),
            "{applied:?}"
        );
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

    /// Issue #983/#986: `EXPR_OPS` used to be a hand-typed 17-entry list
    /// missing every bitwise/shift symbol and every TIP 461 string-ordering
    /// word — a genuinely broken (not just suboptimal) output, since the
    /// unwrapped `set myvar $a << $b` is a 4-argument `set` call (a Tcl
    /// runtime error, `set` takes 1 or 2 args).
    #[test]
    fn bitwise_and_tip461_expressions_are_wrapped_in_expr() {
        let li = LineIndex::new("puts $a << $b");
        let r = extract_variable("puts $a << $b", 5, 13, "shifted", &li).expect("result");
        assert!(
            r.apply("puts $a << $b")
                .contains("set shifted [expr {$a << $b}]"),
            "{:?}",
            r.apply("puts $a << $b")
        );

        let li2 = LineIndex::new("puts $a lt $b");
        let r2 = extract_variable("puts $a lt $b", 5, 13, "ordered", &li2).expect("result");
        assert!(
            r2.apply("puts $a lt $b")
                .contains("set ordered [expr {$a lt $b}]"),
            "{:?}",
            r2.apply("puts $a lt $b")
        );
    }
}
