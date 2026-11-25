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

//! Extract expression binding powers from the pinned C Tcl and Jim sources.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;

const OUTPUT: &str = "rust/tcl-dialect/src/model/generated_precedence.rs";

// C token identifiers are a source-parser vocabulary, independent of releases.
const TOKENS: &[(&str, &str)] = &[
    ("BINARY_PLUS", "+"),
    ("BINARY_MINUS", "-"),
    ("PLUS", "+"),
    ("MINUS", "-"),
    ("MULT", "*"),
    ("DIVIDE", "/"),
    ("MOD", "%"),
    ("LESS", "<"),
    ("GREATER", ">"),
    ("BIT_AND", "&"),
    ("BIT_XOR", "^"),
    ("BIT_OR", "|"),
    ("LEFT_SHIFT", "<<"),
    ("RIGHT_SHIFT", ">>"),
    ("LEQ", "<="),
    ("GEQ", ">="),
    ("EQUAL", "=="),
    ("NEQ", "!="),
    ("AND", "&&"),
    ("OR", "||"),
    ("STREQ", "eq"),
    ("STRNEQ", "ne"),
    ("EXPON", "**"),
    ("IN_LIST", "in"),
    ("NOT_IN_LIST", "ni"),
    ("STR_LT", "lt"),
    ("STR_GT", "gt"),
    ("STR_LEQ", "le"),
    ("STR_GEQ", "ge"),
];

type Rows = BTreeMap<String, (u16, u16)>;

fn c_array<'a>(source: &'a str, declaration: &str) -> Result<&'a str> {
    let after = source
        .split_once(declaration)
        .with_context(|| format!("missing {declaration}"))?
        .1;
    Ok(after
        .split_once('{')
        .context("array open")?
        .1
        .split_once("};")
        .context("array close")?
        .0)
}

fn tcl_modern(source: &str) -> Result<Rows> {
    let enumeration = c_array(source, "enum Precedence")?;
    let identifiers = Regex::new(r"(?m)^\s*(PREC_[A-Z_]+)\s*(?:=\s*([0-9]+))?\s*,?")?;
    let mut levels = BTreeMap::new();
    let mut index = 0u16;
    for row in identifiers.captures_iter(enumeration) {
        index = row
            .get(2)
            .map_or(Ok(index + 1), |value| value.as_str().parse())?;
        levels.insert(row[1].to_owned(), index);
    }
    let floor = *levels.get("PREC_OR").context("PREC_OR")?;
    let table = c_array(source, "static const unsigned char prec[]")?;
    let table = table
        .split_once("/* Binary operator lexemes */")
        .context("binary precedence section")?
        .1
        .split_once("/* Expansion room")
        .context("binary precedence end")?
        .0;
    let entries = Regex::new(r"(PREC_[A-Z_]+),\s*/\*\s*([A-Z_]+)\s*\*/")?;
    let mut rows = Rows::new();
    for row in entries.captures_iter(table) {
        let Some((_, spelling)) = TOKENS.iter().find(|(token, _)| *token == &row[2]) else {
            ensure!(
                matches!(
                    &row[2],
                    "COMMA" | "QUESTION" | "COLON" | "CLOSE_PAREN" | "END"
                ),
                "unknown binary lexeme {}; update the source token vocabulary",
                &row[2]
            );
            continue;
        };
        let raw = levels
            .get(&row[1])
            .context("unknown precedence identifier")?;
        ensure!(*raw >= floor, "binary operator below logical OR");
        let left = 2 * (raw - floor + 2) - u16::from(*spelling == "**");
        let right = left + u16::from(*spelling != "**");
        rows.insert((*spelling).to_owned(), (left, right));
    }
    ensure!(rows.len() >= 23, "incomplete Tcl binary precedence table");
    Ok(rows)
}

fn tcl_84(source: &str) -> Result<Rows> {
    let definition = Regex::new(r"(?m)^#define\s+([A-Z_]+)\s+(\d+)")?;
    let identifiers: BTreeMap<_, _> = definition
        .captures_iter(source)
        .map(|row| Ok((row[1].to_owned(), row[2].parse::<usize>()?)))
        .collect::<Result<_>>()?;
    let strings = c_array(source, "lexemeStrings[]")?;
    // The quote lexeme is the only escaped entry; retain its array position.
    let quoted = if strings.contains(r#""\"""#) {
        strings.replace(r#""\"""#, r#""QUOTE""#)
    } else {
        strings.to_owned()
    };
    let spellings: Vec<_> = Regex::new(r#""([^"\\]*)""#)?
        .captures_iter(&quoted)
        .map(|row| row[1].to_owned())
        .collect();
    let comparison = Regex::new(r"(?:lexeme|infoPtr->lexeme)\s*==\s*([A-Z_]+)")?;
    let child = Regex::new(r"code\s*=\s*(Parse[A-Za-z]+Expr)\(infoPtr\)")?;
    let mut function = "ParseLorExpr".to_owned();
    let mut power = 4;
    let mut rows = Rows::new();
    while function != "ParseUnaryExpr" {
        let begin = format!("\n{function}(infoPtr)");
        let tail = source
            .split_once(&begin)
            .with_context(|| format!("missing {function}"))?
            .1;
        let body = tail.split_once("\n}\n").context("function close")?.0;
        for row in comparison.captures_iter(body) {
            if let Some(index) = identifiers.get(&row[1]) {
                let spelling = spellings.get(*index).context("lexeme string index")?;
                rows.insert(spelling.clone(), (power, power + 1));
            }
        }
        function = child.captures(body).context("recursive-descent child")?[1].to_owned();
        power += 2;
        ensure!(power <= 24, "unexpected parser cycle");
    }
    ensure!(
        rows.len() == 20,
        "expected 20 Tcl 8.4 binary operators, found {}",
        rows.len()
    );
    Ok(rows)
}

fn jim(source: &str) -> Result<Rows> {
    let table = c_array(source, "Jim_ExprOperators[]")?;
    let unary = Regex::new(r#"OPRINIT(?:_ATTR)?\("!""#)?;
    let table = unary
        .find(table)
        .map_or(table, |boundary| &table[..boundary.start()]);
    let pattern =
        Regex::new(r#"OPRINIT(?:_(?:ATTR|LAZY))?\("([^"\\]+)",\s*(\d+),\s*2,\s*([^\n]+)"#)?;
    let mut rows = Rows::new();
    for row in pattern.captures_iter(table) {
        if row[3].contains("OP_FUNC") || matches!(&row[1], "?" | ":") {
            continue;
        }
        let left = 2 * row[2].parse::<u16>()?;
        let right = left + u16::from(!row[3].contains("RIGHT_ASSOC"));
        rows.insert(row[1].to_owned(), (left, right));
    }
    ensure!(rows.len() >= 25, "incomplete Jim precedence table");
    Ok(rows)
}

fn render_table(output: &mut String, name: &str, rows: &Rows) -> Result<()> {
    writeln!(output, "pub(super) const {name}: &[(&str, u16, u16)] = &[")?;
    for (operator, (left, right)) in rows {
        writeln!(output, "    ({operator:?}, {left}, {right}),")?;
    }
    writeln!(output, "];\n")?;
    Ok(())
}

fn extract(root: &std::path::Path) -> Result<String> {
    let mut output =
        String::from("# family\trelease\toperator\tleft binding power\tright binding power\n");
    for version in tcl_dialect::TclVersion::ALL {
        let tree = root.join("tmp").join(version.source_tree_name());
        let source = fs::read_to_string(tree.join("generic").join(
            if version == tcl_dialect::TclVersion::V8_4 {
                "tclParseExpr.c"
            } else {
                "tclCompExpr.c"
            },
        ))?;
        let rows = if version == tcl_dialect::TclVersion::V8_4 {
            tcl_84(&source)?
        } else {
            tcl_modern(&source)?
        };
        for (operator, (left, right)) in rows {
            writeln!(
                output,
                "tcl\t{}\t{operator}\t{left}\t{right}",
                version.version_string()
            )?;
        }
    }
    let jim_tree = root.join("tmp/jimtcl-oracle");
    let mut previous = None;
    for &point in tcl_dialect::model::Family::Jim.releases() {
        let release = point.as_str();
        let result = Command::new("git")
            .args(["show", &format!("{release}:jim.c")])
            .current_dir(&jim_tree)
            .output()?;
        ensure!(
            result.status.success(),
            "read pinned Jim {release}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let rows = jim(std::str::from_utf8(&result.stdout)?)?;
        if previous.as_ref() == Some(&rows) {
            continue;
        }
        previous = Some(rows.clone());
        for (operator, (left, right)) in rows {
            writeln!(output, "jim\t{release}\t{operator}\t{left}\t{right}")?;
        }
    }
    Ok(output)
}

pub fn run(check: bool, verify_sources: bool) -> Result<()> {
    let root = crate::util::repo_root();
    let data_path = root.join("rust/tcl-dialect/data/expression-precedence.tsv");
    if verify_sources {
        let extracted = extract(&root)?;
        if check {
            ensure!(
                fs::read_to_string(&data_path)? == extracted,
                "expression precedence source facts drifted"
            );
        } else {
            crate::util::write_if_changed(&data_path, extracted)?;
        }
    }
    let data = fs::read_to_string(&data_path)?;
    let mut tables: BTreeMap<String, Rows> = BTreeMap::new();
    for line in data
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let fields: Vec<_> = line.split('\t').collect();
        ensure!(fields.len() == 5, "precedence fact requires five fields");
        let name = format!(
            "{}_PRECEDENCE_ROWS_{}",
            fields[0].to_uppercase(),
            fields[1].replace('.', "_")
        );
        ensure!(
            tables
                .entry(name)
                .or_default()
                .insert(
                    fields[2].to_owned(),
                    (fields[3].parse()?, fields[4].parse()?)
                )
                .is_none(),
            "duplicate precedence fact"
        );
    }
    ensure!(
        tables.len() == 9,
        "one precedence table per extracted release"
    );
    let mut output = String::from("// @generated by cargo xtask release-facts; do not edit.\n\n");
    for (name, rows) in tables {
        render_table(&mut output, &name, &rows)?;
    }
    let output = crate::util::format_rust(&output)?.into_bytes();
    let path = root.join(OUTPUT);
    if check {
        if fs::read(&path).ok().as_deref() != Some(&output) {
            bail!("{OUTPUT} is stale; run cargo xtask release-facts");
        }
    } else {
        crate::util::write_if_changed(&path, output)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrecognised_binary_lexemes_fail_source_extraction() {
        let source = "enum Precedence {\nPREC_OR = 7,\nPREC_ADD = 15,\n};\nstatic const unsigned char prec[] = {\n/* Binary operator lexemes */\nPREC_ADD, /* NEW_OPERATOR */\n/* Expansion room */\n};";
        let error = tcl_modern(source).unwrap_err().to_string();
        assert!(
            error.contains("unknown binary lexeme NEW_OPERATOR"),
            "{error}"
        );
    }

    #[test]
    fn jim_reads_associativity_and_ignores_function_rows() {
        let mut input = "Jim_ExprOperators[] = {\n".to_owned();
        for index in 0..25 {
            writeln!(input, "OPRINIT(\"op{index}\", 10, 2, JimExprOpBin),").unwrap();
        }
        input.push_str("OPRINIT_ATTR(\"**\", 120, 2, JimExprOpBin, OP_RIGHT_ASSOC),\nOPRINIT_ATTR(\"pow\", 200, 2, JimExprOpDouble, OP_FUNC),\n};");
        let rows = jim(&input).unwrap();
        assert_eq!(rows.get("**"), Some(&(240, 240)));
        assert_eq!(rows.get("op0"), Some(&(20, 21)));
        assert!(!rows.contains_key("pow"));
        assert!(jim("Jim_ExprOperators[] = {\n};").is_err());
    }
}
