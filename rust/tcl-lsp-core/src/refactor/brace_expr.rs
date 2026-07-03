//! Brace an `expr` command's arguments — convert `expr "..."` or
//! `expr $a + $b` to the braced `expr {$a + $b}` form for safety and
//! performance.

use tcl_registry::CommandRegistry;

use super::{RefactorEdit, Refactoring, find_command_at, token_end_offset};
use crate::code_actions::ActionKind;

/// Convert the `expr` command at `cursor` to braced form, or `None` when the
/// cursor is not on an `expr` command, the expr has no arguments, or it is
/// already braced.
#[must_use]
pub fn brace_expr(
    source: &str,
    cursor: u32,
    registry: &CommandRegistry,
) -> Option<Refactoring> {
    let cmd = find_command_at(source, cursor, Some("expr"), registry)?;

    // Need at least the command word plus one argument.
    if cmd.argv.len() < 2 {
        return None;
    }

    // The raw source span of every argument after `expr`. The first argument
    // starts the span; the closing delimiter belongs to the final argument, so
    // widen its end via `token_end_offset` (the authoritative word-closer) so an
    // empty trailing `{}` / `[]` / `""` is not overshot.
    let first_arg = cmd.argv.get(1)?;
    let last_arg = *cmd.argv.last()?;
    let raw_start = first_arg.span.start();
    let raw_end = token_end_offset(source, last_arg);
    let raw = source.get(raw_start as usize..raw_end as usize)?;

    // Already braced — nothing to do.
    if raw.starts_with('{') && raw.ends_with('}') {
        return None;
    }

    // Unwrap a quoted argument (`"$a + $b"`); otherwise brace the span verbatim.
    let braced = if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        format!("{{{}}}", &raw[1..raw.len() - 1])
    } else {
        format!("{{{raw}}}")
    };

    Some(Refactoring {
        title: "Brace expr for safety and performance".to_owned(),
        edits: vec![RefactorEdit {
            start: raw_start,
            end: raw_end,
            new_text: braced,
        }],
        kind: ActionKind::RefactorRewrite,
        data_group: None,
    })
}

#[cfg(test)]
mod tests {
    use tcl_lexer::LineIndex;

    use super::*;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Byte offset of `(line, character)` for the test cursors below.
    fn offset(source: &str, line: u32, character: u32) -> u32 {
        let li = LineIndex::new(source);
        li.offset_at_utf16(line, tcl_lexer::Utf16Col::new(character), source)
    }

    #[test]
    fn braces_a_quoted_expr() {
        let src = "expr \"$a + $b\"\n";
        let cursor = offset(src, 0, 0);
        let r = brace_expr(src, cursor, &reg()).expect("expr at cursor");
        assert_eq!(r.title, "Brace expr for safety and performance");
        assert_eq!(r.apply(src), "expr {$a + $b}\n");
        assert_eq!(r.edits.len(), 1);
    }

    #[test]
    fn braces_a_bare_expr() {
        let src = "expr $a + $b\n";
        let r = brace_expr(src, offset(src, 0, 0), &reg()).expect("expr at cursor");
        assert_eq!(r.apply(src), "expr {$a + $b}\n");
    }

    #[test]
    fn already_braced_is_none() {
        let src = "expr {$a + $b}\n";
        assert!(brace_expr(src, offset(src, 0, 0), &reg()).is_none());
    }

    #[test]
    fn off_target_is_none() {
        let src = "set x 5\n";
        assert!(brace_expr(src, offset(src, 0, 0), &reg()).is_none());
    }
}
