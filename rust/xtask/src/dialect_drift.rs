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

//! `dialect-drift` — the gate that keeps a document's grammar one value.
//!
//! `docs/design/dialect-profile-model.md` §2.5 settles that a document's
//! grammar is born once, at the ingress (`DocumentEnvironment::grammar`),
//! and threaded from there: every layer that re-reads document text — a
//! proc body, a braced argument, a list word, an `expr` string — reads it
//! under that grammar, never under the default one and never under a
//! grammar re-resolved from a dialect *name*. The JimTcl work found that
//! settlement written and then bypassed: ~150 sites re-lexed bodies with
//! `Lexer::new`, so an iRules body lost its `}{` rule and a Jim body its
//! `$(…)` the moment any pass looked at it a second time.
//!
//! This lint bans the three spellings that reintroduce a second grammar:
//!
//! 1. **A default-grammar re-lex** — `Lexer::new(` or `LexerConfig::default()`
//!    in production code outside `tcl-lexer` itself. A re-lex of document
//!    text takes its config from the nearest context (`State::lexer_config`,
//!    `LexerConfig::for_profile`, `LexerConfig::from_grammar`).
//! 2. **A grammar re-resolved from a name** — `DialectProfile::find(…).grammar`,
//!    `plain_tcl().grammar`, and `LexerConfig::for_dialect(` outside the
//!    resolution owners. A layer that holds a profile reads *its* grammar;
//!    a layer that holds only a name is an ingress and resolves once through
//!    `tcl_registry::model::resolve_environment`.
//! 3. **A bare list split of document text** — `split_list(`,
//!    `split_list_lenient(`, `split_list_jim(` or
//!    `collapse_brace_continuations_str(` inside the document-processing
//!    crates (`tcl-compiler`, `tcl-lsp-core`) outside `WordValueRules`, the
//!    owner keyed by the `brace_backslash_newline` and `list_parse` axes.
//!
//! Text that is genuinely *not* the document's Tcl — a spec DSL, a
//! `tclpkg` manifest, a BIG-IP config extraction, an internal literal —
//! keeps the default grammar and says why: `// dialect-drift-ok: <reason>`
//! on the line or in the comment block directly above it. Test modules
//! (`#[cfg(test)]` and everything after it, `tests/` directories,
//! `tests.rs` files) are exempt: a test may pin the default grammar on
//! purpose.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Files whose job is the thing being linted.
const SANCTIONED_PREFIXES: &[&str] = &[
    // The lexer defines `Lexer::new` and `LexerConfig::default`.
    "rust/tcl-lexer/src/",
    // The resolution owners: the catalogue, the ingress, the grammar table.
    "rust/tcl-dialect/src/",
    "rust/tcl-registry/src/model/ingress.rs",
    "rust/tcl-registry/src/dialects.rs",
    // The word-value owner and the list grammar it is built on.
    "rust/tcl-syntax/src/word_rules.rs",
    "rust/tcl-syntax/src/list.rs",
    // The gates that name the banned spellings in their own source.
    "rust/xtask/src/",
];

/// Crates whose *documents* are Tcl under a dialect, where a bare list
/// split of a word is a second list grammar (rule 3).
const DOCUMENT_CRATES: &[&str] = &["rust/tcl-compiler/src/", "rust/tcl-lsp-core/src/"];

const WAIVER: &str = "dialect-drift-ok:";

struct Rule {
    needles: &'static [&'static str],
    /// `Some(prefixes)` limits the rule to files under those prefixes.
    only_under: Option<&'static [&'static str]>,
    why: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        needles: &["Lexer::new(", "LexerConfig::default()"],
        only_under: None,
        why: "re-lexes under the default grammar; take the config from the context \
              (State::lexer_config / LexerConfig::for_profile / from_grammar)",
    },
    Rule {
        needles: &["LexerConfig::for_dialect("],
        only_under: None,
        why: "re-resolves a grammar from a dialect name; a layer holding a profile reads \
              profile.grammar, an ingress resolves once through resolve_environment",
    },
    Rule {
        needles: &[
            "split_list(",
            "split_list_lenient(",
            "split_list_jim(",
            "collapse_brace_continuations_str(",
        ],
        only_under: Some(DOCUMENT_CRATES),
        why: "splits document text under a fixed list grammar; go through \
              tcl_syntax::word_rules::WordValueRules",
    },
];

/// Scan the workspace; exit non-zero listing every offending site.
/// `check` is accepted for CLI symmetry with the other gates — the lint
/// never rewrites anything, so both modes verify.
pub fn run(_check: bool) -> ExitCode {
    let root = crate::util::repo_root();
    let mut files = Vec::new();
    collect_rs_files(&root.join("rust"), &mut files);
    files.sort();

    let mut report = String::new();
    let mut hits = 0usize;
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if is_exempt_path(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (line_no, snippet, why) in scan(&rel, &text) {
            hits += 1;
            let _ = writeln!(report, "  {rel}:{line_no}: {snippet}\n      -> {why}");
        }
    }

    if hits == 0 {
        println!(
            "dialect-drift: OK (no default-grammar re-lex, name-resolved grammar or bare \
             document list split outside the owners; document text reads under the \
             ingress-resolved grammar)"
        );
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "dialect-drift: {hits} site(s) give a document a second grammar. A document's \
         grammar is resolved once at the ingress (dialect-profile-model.md §2.5) and \
         threaded; a re-read of document text takes it from the nearest context. If the \
         text is genuinely not the document's Tcl (a DSL, a manifest, a fixed literal), \
         mark the line `// dialect-drift-ok: <reason>`:\n{report}"
    );
    ExitCode::FAILURE
}

fn is_exempt_path(rel: &str) -> bool {
    SANCTIONED_PREFIXES.iter().any(|p| rel.starts_with(p))
        || rel.contains("/tests/")
        || rel.ends_with("/tests.rs")
        || rel.contains("/examples/")
        || rel.contains("/benches/")
}

/// Every offending `(line_number, trimmed_line, why)` in `text`.
fn scan<'a>(rel: &str, text: &'a str) -> Vec<(usize, &'a str, &'static str)> {
    let lines: Vec<&str> = text.lines().collect();
    let test_tail = lines
        .iter()
        .position(|l| l.trim_start().starts_with("#[cfg(test)]"))
        .unwrap_or(lines.len());
    let mut out = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(test_tail) {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        let code = code_only(trimmed);
        for rule in RULES {
            if let Some(only) = rule.only_under
                && !only.iter().any(|p| rel.starts_with(p))
            {
                continue;
            }
            if rule.needles.iter().any(|n| calls_free(&code, n)) && !is_waived(&lines, idx) {
                out.push((idx + 1, trimmed, rule.why));
                break;
            }
        }
    }
    out
}

/// Whether `code` calls `needle` as a free function or path (`split_list(`,
/// `list::split_list(`), not as a method on a receiver (`rules.split_list(`)
/// — the receiver form *is* the owner call the list-split rule asks for.
fn calls_free(code: &str, needle: &str) -> bool {
    let mut from = 0;
    while let Some(pos) = code[from..].find(needle) {
        let at = from + pos;
        let preceded_by_dot = at > 0 && code.as_bytes()[at - 1] == b'.';
        if !preceded_by_dot {
            return true;
        }
        from = at + needle.len();
    }
    false
}

/// The code part of a line: everything before a `//` that is not inside a
/// string literal, with the *contents* of string literals elided — a needle
/// quoted in a message or a doc string is a mention, not a call.
fn code_only(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_str = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_str = false;
                out.push('"');
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => {
                in_str = true;
                out.push('"');
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => break,
            _ => out.push(b as char),
        }
        i += 1;
    }
    out
}

/// A waiver is honoured on the flagged line itself, or anywhere in the
/// run of `//` comment lines directly above it (so the reason can be a
/// paragraph). It does not leak past intervening code.
fn is_waived(lines: &[&str], idx: usize) -> bool {
    if lines[idx].contains(WAIVER) {
        return true;
    }
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let t = lines[i].trim_start();
        if !t.starts_with("//") {
            return false;
        }
        if t.contains(WAIVER) {
            return true;
        }
    }
    false
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REL: &str = "rust/tcl-compiler/src/anything.rs";

    #[test]
    fn a_default_relex_is_flagged() {
        let src = "fn f(s: &str) {\n    let t = tcl_lexer::Lexer::new(s).tokenise_all();\n}\n";
        let hits = scan(REL, src);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 2);
    }

    #[test]
    fn a_same_line_waiver_is_honoured() {
        let src = "let t = Lexer::new(s); // dialect-drift-ok: spec DSL, not document Tcl\n";
        assert!(scan(REL, src).is_empty());
    }

    #[test]
    fn a_comment_block_waiver_is_honoured_and_does_not_leak() {
        let src = "// dialect-drift-ok: the manifest grammar is fixed Tcl,\n\
                   // not the document's dialect\n\
                   let a = Lexer::new(s);\n\
                   let b = 1;\n\
                   let c = Lexer::new(s);\n";
        let hits = scan(REL, src);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].0, 5);
    }

    #[test]
    fn the_test_tail_is_exempt() {
        let src = "fn f() {}\n#[cfg(test)]\nmod tests {\n    fn g() { Lexer::new(\"x\"); }\n}\n";
        assert!(scan(REL, src).is_empty());
    }

    #[test]
    fn a_bare_split_is_only_flagged_in_document_crates() {
        let src = "let e = tcl_syntax::list::split_list(word);\n";
        assert_eq!(scan(REL, src).len(), 1);
        assert!(scan("rust/tcl-pkg/src/manifest.rs", src).is_empty());
    }

    #[test]
    fn the_owner_method_call_is_not_a_hit() {
        let src = "let e = rules.split_list(word);\nlet f = self.word_rules().split_list(w);\n";
        assert!(scan(REL, src).is_empty());
        assert_eq!(
            scan(REL, "let e = tcl_syntax::list::split_list(word);\n").len(),
            1
        );
    }

    #[test]
    fn a_mention_in_a_comment_or_string_is_not_a_hit() {
        let src = "// the old form was Lexer::new(s)\nlet m = \"Lexer::new(\";\n";
        assert!(scan(REL, src).is_empty());
    }

    #[test]
    fn sanctioned_and_test_paths_are_exempt() {
        assert!(is_exempt_path("rust/tcl-lexer/src/lexer.rs"));
        assert!(is_exempt_path("rust/tcl-compiler/tests/foo.rs"));
        assert!(is_exempt_path(
            "rust/tcl-compiler/src/analyser/diagnostics/tests.rs"
        ));
        assert!(!is_exempt_path("rust/tcl-compiler/src/lowering/mod.rs"));
    }
}
