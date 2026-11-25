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

//! The naming-owner checks: counted-name facade consumers, and the per-file ledger of hand-written name-delimiter recipes.

use super::manifest::split_row;
use super::{read, read_file};
use anyhow::{Context, Result};
use std::path::Path;

// Native readonly facade consumers keep counted purpose owners. This bounded
// inventory excludes syntax issuers and explicitly selected lexical adapters.
const COUNTED_NAME_FACADES: &[&str] = &[
    "rust/tcl-lsp-core/src/original_declaration.rs",
    "rust/tcl-lsp-core/src/original_indexed_location.rs",
    "rust/tcl-lsp-core/src/original_invocation.rs",
    "rust/tcl-lsp-core/src/method_symbol.rs",
    "rust/tcl-lsp-core/src/namespace_symbol.rs",
    "rust/tcl-lsp-core/src/variable_symbol.rs",
    "rust/tcl-lsp-core/src/variable_symbol/rename.rs",
    "rust/tcl-lsp-core/src/original_command_rename.rs",
    "rust/tcl-lsp-core/src/original_member_rename.rs",
    "rust/tcl-lsp-core/src/math_function_symbol.rs",
    "rust/tcl-lsp-core/src/procedure_symbol.rs",
    "rust/tcl-lsp-core/src/completion/original_workspace.rs",
    "rust/tcl-lsp-core/src/inlay_hints.rs",
];

#[derive(Clone, Copy)]
struct NamingOwnerToken<'a> {
    pub(super) kind: rustc_lexer::TokenKind,
    text: &'a str,
}

fn counted_name_production_tokens(source: &str) -> Vec<NamingOwnerToken<'_>> {
    use rustc_lexer::TokenKind;
    let mut offset = 0;
    let tokens = rustc_lexer::tokenize(source)
        .filter_map(|token| {
            let text = &source[offset..offset + token.len];
            offset += token.len;
            (!matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
            ))
            .then_some(NamingOwnerToken {
                kind: token.kind,
                text,
            })
        })
        .collect::<Vec<_>>();
    let mut production = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        // Remove only the cfg(test) item itself, then resume any later
        // production items. Literals/comments cannot fabricate this attribute.
        if tokens.get(index..index + 7).is_some_and(|attribute| {
            attribute
                .iter()
                .map(|token| token.text)
                .eq(["#", "[", "cfg", "(", "test", ")", "]"])
        }) {
            index += 7;
            let mut braces = 0usize;
            while let Some(token) = tokens.get(index) {
                index += 1;
                match token.kind {
                    TokenKind::OpenBrace => braces += 1,
                    TokenKind::CloseBrace if braces == 1 => break,
                    TokenKind::CloseBrace => braces = braces.saturating_sub(1),
                    TokenKind::Semi if braces == 0 => break,
                    _ => {}
                }
            }
            continue;
        }
        production.push(tokens[index]);
        index += 1;
    }
    production
}

fn counted_name_delimiter(literal: &str) -> bool {
    match syn::parse_str::<syn::Lit>(literal) {
        Ok(syn::Lit::Str(value)) => matches!(value.value().as_str(), "::" | ":" | "\0"),
        Ok(syn::Lit::Char(value)) => matches!(value.value(), ':' | '\0'),
        Ok(syn::Lit::Byte(value)) => matches!(value.value(), b':' | 0),
        _ => false,
    }
}

fn counted_name_zero(literal: &str) -> bool {
    match syn::parse_str::<syn::Lit>(literal) {
        Ok(syn::Lit::Int(value)) => value.base10_digits() == "0",
        Ok(syn::Lit::Char(value)) => value.value() == '\0',
        Ok(syn::Lit::Byte(value)) => value.value() == 0,
        _ => false,
    }
}

pub(super) fn counted_name_local_recipes(source: &str) -> Vec<String> {
    use rustc_lexer::TokenKind;
    let tokens = counted_name_production_tokens(source);
    let mut problems = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Ident && token.text == "from_utf8_lossy" {
            problems.push("lossy resident name reconstruction".to_owned());
        }
        if token.kind != TokenKind::Dot {
            continue;
        }
        let Some(method) = tokens.get(index + 1) else {
            continue;
        };
        if tokens
            .get(index + 2)
            .is_none_or(|open| open.kind != TokenKind::OpenParen)
        {
            continue;
        }
        if matches!(
            method.text,
            "split"
                | "rsplit"
                | "split_once"
                | "rsplit_once"
                | "find"
                | "rfind"
                | "trim_start_matches"
                | "trim_end_matches"
        ) && tokens
            .get(index + 3)
            .is_some_and(|argument| counted_name_delimiter(argument.text))
        {
            problems.push(format!(
                "local counted-name delimiter recipe: {}",
                method.text
            ));
        }
        if matches!(method.text, "split" | "take_while" | "position") {
            let mut depth = 1usize;
            let mut end = index + 3;
            while let Some(next) = tokens.get(end) {
                match next.kind {
                    TokenKind::OpenParen => depth += 1,
                    TokenKind::CloseParen => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                end += 1;
            }
            if tokens[index + 3..end].windows(3).any(|expression| {
                expression[0].kind == TokenKind::Eq
                    && expression[1].kind == TokenKind::Eq
                    && counted_name_zero(expression[2].text)
            }) {
                problems.push(format!(
                    "local zero-terminated name recipe: {}",
                    method.text
                ));
            }
        }
    }
    problems
}

pub(super) fn validate_counted_name_facade_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for path in COUNTED_NAME_FACADES {
        for problem in counted_name_local_recipes(&read(root, path)?) {
            problems.push(format!(
                "native readonly naming facade {path}: {problem}; use the shared purpose owner"
            ));
        }
    }
    Ok(problems)
}

/// The per-file count of qualified-name delimiter recipes outside the naming
/// owner. Generated; a count may only fall.
const NAME_DELIMITER_LEDGER: &str = "docs/generated/naming-delimiters.md";

/// Production source trees the delimiter ledger covers.
const NAME_DELIMITER_ROOTS: &[&str] = &["rust", "runtime/rust/src"];

/// Sites the recogniser matches where the joined text is not a Tcl name. Each
/// row is subtracted from its file's count and listed in the ledger.
pub(super) const NAME_DELIMITER_OTHER_TEXT: &[(&str, usize, &str)] = &[
    (
        "rust/f5-cli/src/commands/explain_flow.rs",
        1,
        "report label of a rule path and an event",
    ),
    (
        "rust/f5-cli/src/commands/irule.rs",
        1,
        "input label of a file and a rule path",
    ),
    (
        "rust/tcl-bigip/src/parser/driver.rs",
        1,
        "configuration object key of module, type and identifier",
    ),
    (
        "rust/tcl-bigip/src/redact.rs",
        1,
        "hash input of a seed and an address range",
    ),
];

/// String methods that split, find or strip a qualified name by its separator.
const NAME_DELIMITER_METHODS: &[&str] = &[
    "split",
    "rsplit",
    "splitn",
    "rsplitn",
    "split_once",
    "rsplit_once",
    "find",
    "rfind",
    "contains",
    "matches",
    "starts_with",
    "ends_with",
    "strip_prefix",
    "strip_suffix",
    "trim_start_matches",
    "trim_end_matches",
];

/// String methods that trim colons off a name.
const NAME_COLON_TRIM_METHODS: &[&str] =
    &["trim_start_matches", "trim_end_matches", "trim_matches"];

/// Whether `literal` is a single colon as a character, string or byte.
fn colon_literal(literal: &str) -> bool {
    match syn::parse_str::<syn::Lit>(literal) {
        Ok(syn::Lit::Char(value)) => value.value() == ':',
        Ok(syn::Lit::Str(value)) => value.value() == ":",
        Ok(syn::Lit::Byte(value)) => value.value() == b':',
        Ok(syn::Lit::ByteStr(value)) => value.value() == b":",
        _ => false,
    }
}

/// Whether `literal` is exactly the namespace separator.
fn name_separator_literal(literal: &str) -> bool {
    match syn::parse_str::<syn::Lit>(literal) {
        Ok(syn::Lit::Str(value)) => value.value() == "::",
        Ok(syn::Lit::ByteStr(value)) => value.value() == b"::",
        _ => false,
    }
}

/// Whether `literal` is a format template that only joins holes with the
/// separator (`"::{}"`, `"{}::{}"`, `"::{ns}::{name}"`).
pub(super) fn name_join_template(literal: &str) -> bool {
    let Ok(syn::Lit::Str(value)) = syn::parse_str::<syn::Lit>(literal) else {
        return false;
    };
    let value = value.value();
    let mut rest = value.strip_prefix("::").unwrap_or(&value);
    let mut holes = 0usize;
    loop {
        let Some(after) = rest.strip_prefix('{') else {
            return false;
        };
        let Some(close) = after.find('}') else {
            return false;
        };
        if after[..close].contains('{') {
            return false;
        }
        holes += 1;
        rest = &after[close + 1..];
        if rest.is_empty() {
            return value.contains("::") && holes > 0;
        }
        let Some(next) = rest.strip_prefix("::") else {
            return false;
        };
        rest = next;
    }
}

/// How many times production code in `source` splits, searches, strips or
/// joins a qualified name by a hand-written separator.
pub(super) fn name_delimiter_recipes(source: &str) -> usize {
    use rustc_lexer::TokenKind;
    let tokens = counted_name_production_tokens(source);
    let mut count = 0;
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token.kind, TokenKind::Literal { .. }) && name_join_template(token.text) {
            count += 1;
        }
        if token.kind == TokenKind::Dot
            && tokens
                .get(index + 1)
                .is_some_and(|method| NAME_DELIMITER_METHODS.contains(&method.text))
            && tokens
                .get(index + 2)
                .is_some_and(|open| open.kind == TokenKind::OpenParen)
            && tokens
                .get(index + 3)
                .is_some_and(|argument| name_separator_literal(argument.text))
        {
            count += 1;
        }
        if token.kind == TokenKind::Dot
            && tokens
                .get(index + 1)
                .is_some_and(|method| NAME_COLON_TRIM_METHODS.contains(&method.text))
            && tokens
                .get(index + 2)
                .is_some_and(|open| open.kind == TokenKind::OpenParen)
            && tokens
                .get(index + 3)
                .is_some_and(|argument| colon_literal(argument.text))
        {
            count += 1;
        }
    }
    count
}

/// Whether `relative` is production source the ledger counts: not the naming
/// owner, not this gate, and not a test tree or test-only file.
pub(super) fn counts_name_delimiters(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or(relative);
    Path::new(relative)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        && relative.contains("/src/")
        && !relative.starts_with("rust/tcl-syntax/src/naming")
        && !relative.starts_with("rust/xtask/")
        && !relative.contains("/tests/")
        && name != "tests.rs"
        && !name.ends_with("_tests.rs")
        && !crate::util::test_sources::is_test_module_file(relative)
        && !name.ends_with("_test.rs")
}

fn collect_rust_sources(root: &Path, relative: &str, found: &mut Vec<String>) -> Result<()> {
    let directory = root.join(relative);
    let entries = std::fs::read_dir(&directory)
        .with_context(|| format!("read owner-resolution directory `{relative}`"))?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = format!("{relative}/{name}");
        if entry.file_type()?.is_dir() {
            if name != "target" && !name.starts_with('.') {
                collect_rust_sources(root, &child, found)?;
            }
        } else if counts_name_delimiters(&child) {
            found.push(child);
        }
    }
    Ok(())
}

pub(super) fn production_source_paths(root: &Path) -> Result<Vec<String>> {
    let mut sources = Vec::new();
    for tree in NAME_DELIMITER_ROOTS {
        collect_rust_sources(root, tree, &mut sources)?;
    }
    Ok(sources)
}

fn workspace_name_delimiter_counts(
    root: &Path,
) -> Result<std::collections::BTreeMap<String, usize>> {
    let mut counts = std::collections::BTreeMap::new();
    for path in production_source_paths(root)? {
        let other_text = NAME_DELIMITER_OTHER_TEXT
            .iter()
            .find(|(file, _, _)| *file == path)
            .map_or(0, |(_, sites, _)| *sites);
        // Every file is visited on its own here, so each is read alone.
        let count = name_delimiter_recipes(&read_file(root, &path)?).saturating_sub(other_text);
        if count > 0 {
            counts.insert(path, count);
        }
    }
    Ok(counts)
}

pub(super) fn render_name_delimiter_ledger(
    counts: &std::collections::BTreeMap<String, usize>,
) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "<!-- Generated by `cargo xtask owner-resolution`; do not edit. -->\n\n\
         # Qualified-name delimiter recipes outside the naming owner\n\n\
         `tcl_syntax::naming` owns how a Tcl name is split, tested and joined. Each row is a \
         production file that still does it by hand: a string method given the literal `::`, \
         a colon trim, or a format template that only joins holes with `::`. A count may only fall; a new \
         site fails `cargo xtask owner-resolution`. Call the owner instead, then rerun the \
         gate, which lowers the count here.\n\n| File | Sites |\n|---|---|\n",
    );
    for (path, count) in counts {
        let _ = writeln!(out, "| `{path}` | {count} |");
    }
    let _ = writeln!(
        out,
        "\n{} site(s) in {} file(s).",
        counts.values().sum::<usize>(),
        counts.len()
    );
    out.push_str(
        "\n## Matches that are not Tcl names\n\nThe gate subtracts these; the joined text is \
         some other key or label.\n\n| File | Sites | Text |\n|---|---|---|\n",
    );
    for (path, sites, text) in NAME_DELIMITER_OTHER_TEXT {
        let _ = writeln!(out, "| `{path}` | {sites} | {text} |");
    }
    out
}

pub(super) fn parse_name_delimiter_ledger(
    ledger: &str,
) -> std::collections::BTreeMap<String, usize> {
    ledger
        .lines()
        .filter_map(|line| {
            let cells = split_row(line);
            let [path, count] = cells.as_slice() else {
                return None;
            };
            let path = path.strip_prefix('`')?.strip_suffix('`')?;
            Some((path.to_owned(), count.parse().ok()?))
        })
        .collect()
}

/// Hold every production file to its recorded count of hand-written name
/// delimiter recipes. A rise, or a first site in an unrecorded file, is a
/// problem; a fall rewrites the ledger.
pub(super) fn validate_workspace_name_delimiters(root: &Path) -> Result<Vec<String>> {
    let counts = workspace_name_delimiter_counts(root)?;
    let recorded = std::fs::read_to_string(root.join(NAME_DELIMITER_LEDGER))
        .map(|ledger| parse_name_delimiter_ledger(&ledger))
        .ok();
    let Some(recorded) = recorded else {
        std::fs::write(
            root.join(NAME_DELIMITER_LEDGER),
            render_name_delimiter_ledger(&counts),
        )
        .with_context(|| format!("write `{NAME_DELIMITER_LEDGER}`"))?;
        return Ok(Vec::new());
    };
    let problems: Vec<String> = counts
        .iter()
        .filter_map(|(path, count)| {
            let allowed = recorded.get(path).copied().unwrap_or(0);
            (*count > allowed).then(|| {
                format!(
                    "{path} splits or joins a qualified name by hand {count} time(s), {allowed} \
                     recorded in {NAME_DELIMITER_LEDGER}; call tcl_syntax::naming"
                )
            })
        })
        .collect();
    if problems.is_empty() && counts != recorded {
        std::fs::write(
            root.join(NAME_DELIMITER_LEDGER),
            render_name_delimiter_ledger(&counts),
        )
        .with_context(|| format!("write `{NAME_DELIMITER_LEDGER}`"))?;
    }
    Ok(problems)
}
