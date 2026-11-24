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

use rustc_lexer::TokenKind;

use super::Gate;

pub(super) const GRAMMARS: &[&str] = &[
    "completion-codes",
    "docstrings",
    "elements",
    "encodings",
    "expressions",
    "globs",
    "indices",
    "interp-create",
    "interp-paths",
    "levels",
    "lists",
    "numbers",
    "options",
    "paths",
    "pipelines",
    "positions",
    "regex",
    "script-files",
    "scripts",
    "subcommands",
    "tclindex",
    "tcllibpath",
    "tsv",
    "value-grammars",
    "variables",
    "versions",
    "waivers",
    "words",
];

const ANALYSIS: &[&str] = &[
    "tcl-cli-support",
    "tcl-codegen",
    "tcl-compiler",
    "tcl-diagram",
    "tcl-explorer",
    "tcl-irule-test",
    "tcl-irules",
    "tcl-lsp-core",
    "tcl-lsp-server",
    "tcl-mcp",
];

const GRAMMAR_CONSUMERS: &[&str] = &[
    "tcl-bytecode",
    "tcl-cli",
    "tcl-cli-support",
    "tcl-cmd-core",
    "tcl-codegen",
    "tcl-compiler",
    "tcl-diagram",
    "tcl-explorer",
    "tcl-expr",
    "tcl-irule-test",
    "tcl-irules",
    "tcl-lexer",
    "tcl-lsp-core",
    "tcl-lsp-db",
    "tcl-lsp-server",
    "tcl-mcp",
    "tcl-numeric",
    "tcl-pkg",
    "tcl-pkg-model",
    "tcl-regex",
    "tcl-registry",
    "tcl-spectcl",
    "tcl-sslictcl",
    "tcl-syntax",
    "tcl-value-syntax",
    "tcl-vm",
];

pub(super) fn crates(gate: Gate) -> &'static [&'static str] {
    match gate {
        Gate::Walker | Gate::Memo => ANALYSIS,
        Gate::Grammar | Gate::Release => GRAMMAR_CONSUMERS,
    }
}

#[derive(Clone, Copy)]
struct Token<'a> {
    text: &'a str,
    line: usize,
}

fn tokens(source: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut offset = 0;
    let mut line = 1;
    for token in rustc_lexer::tokenize(source) {
        let end = offset + token.len;
        let text = &source[offset..end];
        if !matches!(
            token.kind,
            TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
        ) {
            out.push(Token { text, line });
        }
        line += text.bytes().filter(|byte| *byte == b'\n').count();
        offset = end;
    }
    out
}

pub(super) fn declares(source: &str, symbol: &str) -> bool {
    let tokens = tokens(source);
    tokens.windows(2).any(|pair| {
        matches!(
            pair[0].text,
            "fn" | "struct" | "enum" | "type" | "const" | "static" | "trait" | "mod"
        ) && pair[1].text == symbol
    })
}

pub(super) fn test_declares(source: &str, symbol: &str) -> bool {
    let tokens = tokens(source);
    tokens.windows(6).any(|tokens| {
        tokens
            .iter()
            .map(|token| token.text)
            .eq(["#", "[", "test", "]", "fn", symbol])
    })
}

pub(super) struct Hit<'a> {
    pub(super) rule: &'a str,
    pub(super) site: String,
    pub(super) line: usize,
}

pub(super) fn scan<'a>(gate: Gate, path: &str, text: &'a str) -> Vec<Hit<'a>> {
    let tokens = tokens(text);
    let test_ranges = test_ranges(&tokens);
    let mut scope = "<module>";
    let mut hits = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        if test_ranges.iter().any(|range| range.contains(&at)) {
            continue;
        }
        let before = look(&tokens, at.checked_sub(1));
        let after = |n| look(&tokens, Some(at + n));
        if matches!(token.text, "fn" | "struct" | "static" | "const" | "type")
            && after(1).is_some_and(identifier)
        {
            scope = after(1).unwrap_or(scope);
        }
        let candidate = match gate {
            Gate::Grammar => primitive(token.text, before, after).or_else(|| {
                let path = at.checked_sub(3).and_then(|start| tokens.get(start..at))?;
                if !path.iter().map(|token| token.text).eq(["str", ":", ":"]) {
                    return None;
                }
                let rule = whitespace_rule(token.text)?;
                Some((rule, token.text))
            }),
            Gate::Walker => walker(path, token.text, before, after),
            Gate::Memo => memo(token.text, after),
            Gate::Release => release(token.text, after, &tokens[at..]),
        };
        if let Some((rule, spelling)) = candidate {
            hits.push(Hit {
                rule,
                site: format!("{scope}:{spelling}"),
                line: token.line,
            });
        }
    }
    hits
}

fn test_ranges(tokens: &[Token<'_>]) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    for (start, attribute) in tokens.windows(5).enumerate() {
        if !attribute
            .iter()
            .map(|token| token.text)
            .eq(["#", "[", "cfg", "(", "test"])
        {
            continue;
        }
        let rest = &tokens[start + 5..];
        if !rest.iter().take(2).map(|token| token.text).eq([")", "]"]) {
            continue;
        }
        let item = &rest[2..];
        let Some(open) = item
            .iter()
            .position(|token| matches!(token.text, "{" | ";"))
        else {
            continue;
        };
        if item[open].text != "{" || !item[..open].iter().any(|token| token.text == "mod") {
            continue;
        }
        let mut depth = 1;
        for (offset, token) in item[open + 1..].iter().enumerate() {
            match token.text {
                "{" => depth += 1,
                "}" => depth -= 1,
                _ => continue,
            }
            if depth == 0 {
                ranges.push(start..start + 7 + open + offset + 2);
                break;
            }
        }
    }
    ranges
}

fn look<'a>(tokens: &[Token<'a>], at: Option<usize>) -> Option<&'a str> {
    at.and_then(|at| tokens.get(at)).map(|token| token.text)
}

fn identifier(text: &str) -> bool {
    text.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
}

fn primitive<'a>(
    word: &'a str,
    before: Option<&str>,
    after: impl Fn(usize) -> Option<&'a str>,
) -> Option<(&'static str, &'a str)> {
    if before != Some(".") || after(1) != Some("(") {
        return None;
    }
    let grammar = match word {
        "contains"
            if matches!(after(2), Some("'('" | "\"(\""))
                || (after(2) == Some("&") && after(3) == Some("b'('")) =>
        {
            "elements"
        }
        "ends_with" if matches!(after(2), Some("'\\\\'" | "\"\\\\\"")) => "words",
        "starts_with" if matches!(after(2), Some("'|'" | "\"|\"" | "b\"|\"")) => "pipelines",
        "chars"
            if (2..=5)
                .map(after)
                .eq([Some(")"), Some("."), Some("count"), Some("(")]) =>
        {
            "encodings"
        }
        _ => whitespace_rule(word)?,
    };
    Some((grammar, word))
}

fn whitespace_rule(word: &str) -> Option<&'static str> {
    match word {
        "split_whitespace" | "split_ascii_whitespace" => Some("lists"),
        "trim" | "trim_start" | "trim_end" | "trim_ascii" | "trim_ascii_start"
        | "trim_ascii_end" => Some("scripts"),
        _ => None,
    }
}

fn walker<'a>(
    path: &str,
    word: &'a str,
    before: Option<&str>,
    after: impl Fn(usize) -> Option<&'a str>,
) -> Option<(&'static str, &'a str)> {
    if path.starts_with("rust/tcl-compiler/src/command_binding/")
        || path == "rust/tcl-compiler/src/command_binding.rs"
        || path.starts_with("rust/tcl-compiler/src/var_resolve/")
        || path == "rust/tcl-compiler/src/var_resolve.rs"
    {
        return None;
    }
    if before == Some("fn") && matches!(after(1), Some("(" | "<")) {
        if word.starts_with("walk_") || word.starts_with("visit_") {
            return Some(("source-walk", word));
        }
        if word.starts_with("resolve_")
            && [
                "command",
                "variable",
                "namespace",
                "binding",
                "proc",
                "name",
            ]
            .iter()
            .any(|fact| word.contains(fact))
        {
            let fact = if word.contains("variable") {
                "variable-resolution"
            } else {
                "name-resolution"
            };
            return Some((fact, word));
        }
    }
    if after(1) == Some("(")
        && before != Some("fn")
        && (word.starts_with("segment_commands")
            || word.starts_with("native_script_words_in")
            || matches!(
                word,
                "walk_executable_commands" | "visit_analysis_executable_commands"
            ))
    {
        return Some(("source-reader", word));
    }
    None
}

fn memo<'a>(
    word: &'a str,
    after: impl Fn(usize) -> Option<&'a str>,
) -> Option<(&'static str, &'a str)> {
    let found = match word {
        "OnceLock" | "OnceCell" | "LazyLock" | "LazyCell" => after(1) == Some("<"),
        "Mutex" | "RwLock" => {
            after(1) == Some("<")
                && (2..8).map(after).any(|name| {
                    matches!(name, Some("Option" | "HashMap" | "BTreeMap" | "FxHashMap"))
                })
        }
        "thread_local" => after(1) == Some("!"),
        "Box" => (1..=4)
            .map(after)
            .eq([Some(":"), Some(":"), Some("leak"), Some("(")]),
        _ => false,
    };
    found.then_some(("analysis-memo", word))
}

fn release<'a>(
    word: &'a str,
    after: impl Fn(usize) -> Option<&'a str>,
    tail: &[Token<'_>],
) -> Option<(&'static str, &'a str)> {
    if !matches!(word, "const" | "static") {
        return None;
    }
    let name = after(1)?;
    if !identifier(name) || !matches!(after(2), Some(":" | "=")) {
        return None;
    }
    let fields = tail.iter().take_while(|token| token.text != ";");
    let release_key = fields.clone().any(|token| {
        matches!(
            token.text,
            "TclVersion" | "TclRelease" | "V8_4" | "V8_5" | "V8_6" | "V9_0" | "V9_1"
        )
    });
    let table = fields.clone().any(|token| matches!(token.text, "[" | "{"));
    let fact_table = [
        "PRECEDENCE",
        "OPERATORS",
        "MATH_FUNCTION",
        "BUNDLED_PACKAGE",
        "RELEASE_TABLE",
    ]
    .iter()
    .any(|prefix| name.contains(prefix));
    (table && (release_key || fact_table)).then_some(("release-facts", name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_new_parallel_walkers_and_resolvers() {
        let text = "fn walk_source() {} fn resolve_command_here() {}";
        let hits = scan(Gate::Walker, "consumer.rs", text);
        assert_eq!(
            hits.iter().map(|hit| hit.rule).collect::<Vec<_>>(),
            ["source-walk", "name-resolution"]
        );
        assert!(
            scan(
                Gate::Walker,
                "rust/tcl-compiler/src/command_binding.rs",
                text
            )
            .is_empty()
        );
    }

    #[test]
    fn finds_new_memos_outside_the_database() {
        let hits = scan(
            Gate::Memo,
            "analysis.rs",
            "static MEMO: OnceLock<Mutex<HashMap<K, V>>> = OnceLock::new();",
        );
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|hit| hit.rule == "analysis-memo"));
    }

    #[test]
    fn finds_hand_written_operator_tables() {
        let text = "const OPERATOR_PRECEDENCE: &[(TclVersion, u8)] = &[(TclVersion::V8_6, 4)];";
        let hits = scan(Gate::Release, "compiler.rs", text);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rule, "release-facts");
    }

    #[test]
    fn an_inline_test_module_cannot_hide_later_production_code() {
        let text =
            "#[cfg(test)] mod tests { fn test() { text.trim(); } } fn after() { text.trim(); }";
        let hits = scan(Gate::Grammar, "consumer.rs", text);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].site, "after:trim");
    }

    #[test]
    fn finds_tcl_host_primitives_across_lines_and_ignores_literals() {
        let text = r#"
fn handle(tcl: &str) {
    tcl
        .split_whitespace();
    tcl.trim();
    tcl.contains('(');
    tcl.ends_with('\\');
    tcl.starts_with('|');
    tcl.chars().count();
    bytes.contains(&b'(');
    bytes.starts_with(b"|");
    words.iter().map(str::trim);
    let example = "tcl.trim();";
    // tcl.trim();
}
"#;
        let hits = scan(Gate::Grammar, "consumer.rs", text);
        assert_eq!(
            hits.iter().map(|hit| hit.rule).collect::<Vec<_>>(),
            [
                "lists",
                "scripts",
                "elements",
                "words",
                "pipelines",
                "encodings",
                "elements",
                "pipelines",
                "scripts",
            ]
        );
    }
}
