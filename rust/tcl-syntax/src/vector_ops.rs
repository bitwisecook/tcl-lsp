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

//! Row syntax shared by the conformance vector files.
//!
//! Every vector file is pipe-separated with `#` comment lines, and the
//! setup column of the variable and namespace-operation files is a
//! comma-separated list of *mini-ops* — `kind(argument)` — such as
//! `ns(::a),var(::a::x=1),decl(variable x)`.  Arguments may themselves
//! contain commas and parentheses, so the split tracks parenthesis depth.

/// Split one row into its trimmed pipe-separated fields.
#[must_use]
pub fn split_row(line: &str) -> Vec<&str> {
    line.split('|').map(str::trim).collect()
}

/// Split a mini-op column into `(kind, argument)` pairs.
///
/// `-` is the empty column.  Whitespace around an op is ignored; the
/// argument is taken verbatim between the outermost parentheses so a Tcl
/// fragment inside it keeps its spacing.
///
/// # Errors
/// Returns a human-readable message for an op that is not
/// `kind(argument)`, or whose parentheses do not balance.
pub fn split_ops(field: &str) -> Result<Vec<(String, String)>, String> {
    let field = field.trim();
    if field == "-" || field.is_empty() {
        return Ok(Vec::new());
    }
    let mut ops = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;
    let mut chunks = Vec::new();
    for (offset, ch) in field.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| format!("unbalanced `)` in ops column {field:?}"))?;
            }
            ',' if depth == 0 => {
                chunks.push(&field[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(format!("unbalanced `(` in ops column {field:?}"));
    }
    chunks.push(&field[start..]);
    for chunk in chunks {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let open = chunk
            .find('(')
            .ok_or_else(|| format!("op {chunk:?} is not `kind(argument)`"))?;
        let close = chunk
            .strip_suffix(')')
            .ok_or_else(|| format!("op {chunk:?} does not end with `)`"))?;
        ops.push((
            chunk[..open].trim().to_owned(),
            close[open + 1..].trim().to_owned(),
        ));
    }
    Ok(ops)
}

/// Split `head rest` into the first whitespace-separated word and the
/// remainder, with an empty remainder when there is only one word.
#[must_use]
pub fn split_head(argument: &str) -> (&str, &str) {
    match argument.trim().split_once(char::is_whitespace) {
        Some((head, rest)) => (head, rest.trim_start()),
        None => (argument.trim(), ""),
    }
}

#[cfg(test)]
mod tests {
    use super::{split_head, split_ops, split_row};

    #[test]
    fn a_dash_column_holds_no_ops() {
        assert_eq!(split_ops("-").expect("parses"), Vec::new());
        assert_eq!(split_ops("").expect("parses"), Vec::new());
    }

    #[test]
    fn ops_split_on_top_level_commas_only() {
        let ops = split_ops("ns(::a), var(::a::x=1), eval(list a, b)").expect("parses");
        assert_eq!(
            ops,
            vec![
                ("ns".to_owned(), "::a".to_owned()),
                ("var".to_owned(), "::a::x=1".to_owned()),
                ("eval".to_owned(), "list a, b".to_owned()),
            ]
        );
    }

    #[test]
    fn nested_parentheses_stay_inside_one_op() {
        let ops = split_ops("eval(set a(1) [list x(2), y])").expect("parses");
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].1, "set a(1) [list x(2), y]");
    }

    #[test]
    fn unbalanced_parentheses_are_rejected() {
        assert!(split_ops("ns(::a").is_err());
        assert!(split_ops("ns(::a))").is_err());
        assert!(split_ops("bare").is_err());
    }

    #[test]
    fn rows_split_on_pipes_and_trim() {
        assert_eq!(split_row(" a | b |c "), vec!["a", "b", "c"]);
    }

    #[test]
    fn head_split_takes_the_first_word() {
        assert_eq!(split_head("::ns  a b"), ("::ns", "a b"));
        assert_eq!(split_head("::ns"), ("::ns", ""));
    }
}
