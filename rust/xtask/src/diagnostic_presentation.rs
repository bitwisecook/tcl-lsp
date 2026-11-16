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
//! `diagnostic-presentation` — the #2426 drift gate.
//!
//! A diagnostic's message, a fix's description or title, and an edit's
//! replacement text are presentation. Consumers take names, kinds and
//! subjects from the typed payload the emitter retained
//! ([`docs/design/contracts/diagnostic-subjects.md`]), never by parsing the
//! rendered text. This gate flags production code that calls a text-parsing
//! method on one of those fields, in the Rust tree and the VS Code adapter.
//!
//! A reviewed presentation-only read (punctuation joining, newline
//! normalisation of an edit before it is applied) carries a
//! `// presentation-ok: <reason>` comment on the flagged line or one of the
//! four lines above it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Presentation fields of diagnostics, fixes, notices and edits.
const FIELDS: &[&str] = &["message", "description", "title", "new_text", "newText"];

/// Methods that recover structure from text. Whole-value formatting
/// (`trim`, `replace`, `to_owned`) is not a parse.
const RUST_METHODS: &[&str] = &[
    "contains",
    "starts_with",
    "ends_with",
    "split",
    "splitn",
    "rsplit",
    "split_once",
    "rsplit_once",
    "split_whitespace",
    "find",
    "rfind",
    "strip_prefix",
    "strip_suffix",
    "matches",
    "match_indices",
    "char_indices",
];

const TS_METHODS: &[&str] = &[
    "includes",
    "startsWith",
    "endsWith",
    "split",
    "indexOf",
    "lastIndexOf",
    "match",
    "matchAll",
    "search",
];

/// Test-only source trees that a `#[cfg(test)] mod` pulls in by path.
const TEST_ONLY_DIRS: &[&str] = &["rust/tcl-compiler/src/analyser/diagnostics/fp/"];

pub fn run(_check: bool) -> ExitCode {
    let root = crate::util::repo_root();
    let mut files = Vec::new();
    collect(&root.join("rust"), &mut files);
    collect(&root.join("runtime/rust/src"), &mut files);
    collect(&root.join("editors/vscode/src"), &mut files);

    let mut report = String::new();
    let mut hits = 0usize;
    for path in files {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if is_test_path(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let typescript = path.extension().is_some_and(|e| e == "ts");
        for (line_no, line) in flagged_lines(&text, typescript) {
            hits += 1;
            let snippet: String = line.trim().chars().take(96).collect();
            let _ = writeln!(report, "  {rel}:{line_no}: {snippet}");
        }
    }

    if hits == 0 {
        println!("diagnostic-presentation: OK (no consumer parses diagnostic presentation)");
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "diagnostic-presentation: {hits} site(s) parse a diagnostic message, fix \
         description or replacement text — read the emitter's typed payload \
         instead, or mark a reviewed presentation-only read with \
         `// presentation-ok: <reason>`:\n{report}"
    );
    ExitCode::FAILURE
}

fn is_test_path(rel: &str) -> bool {
    rel.contains("/tests/")
        || rel.contains("/test/")
        || rel.ends_with("/tests.rs")
        || rel.ends_with("_tests.rs")
        || crate::util::test_sources::is_test_module_file(rel)
        || rel.ends_with(".test.ts")
        || rel == "rust/xtask/src/diagnostic_presentation.rs"
        || TEST_ONLY_DIRS.iter().any(|dir| rel.starts_with(dir))
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == "node_modules" || n == "out")
            {
                continue;
            }
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs" || e == "ts") {
            out.push(path);
        }
    }
}

/// `(1-based line, line)` for each unwaived production line that parses a
/// presentation field. Rust lines after the file's test module are skipped.
fn flagged_lines(text: &str, typescript: bool) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    let end = if typescript {
        lines.len()
    } else {
        crate::util::test_module_start(&lines).unwrap_or(lines.len())
    };
    let methods = if typescript { TS_METHODS } else { RUST_METHODS };
    lines[..end]
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            let code = line.trim_start();
            !code.starts_with("//") && !code.starts_with('*') && parses_presentation(code, methods)
        })
        .filter(|(idx, _)| !waived(&lines, *idx))
        .map(|(idx, line)| (idx + 1, *line))
        .collect()
}

/// Whether `code` reads `.FIELD.METHOD(` (optionally through `.as_str()`).
fn parses_presentation(code: &str, methods: &[&str]) -> bool {
    FIELDS.iter().any(|field| {
        let needle = format!(".{field}.");
        code.match_indices(&needle).any(|(at, _)| {
            let before = code[..at].chars().next_back();
            if before.is_some_and(|c| c == '.') {
                return false;
            }
            let rest = &code[at + needle.len()..];
            let rest = rest.strip_prefix("as_str().").unwrap_or(rest);
            methods.iter().any(|method| {
                rest.strip_prefix(method)
                    .is_some_and(|tail| tail.starts_with('('))
            })
        })
    })
}

fn waived(lines: &[&str], idx: usize) -> bool {
    (idx.saturating_sub(4)..=idx)
        .filter_map(|i| lines.get(i))
        .any(|l| l.contains("presentation-ok:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_a_quoted_name_recovered_from_a_message() {
        let src = "let name = d.message.split('\\'').nth(1);\n";
        assert_eq!(flagged_lines(src, false).len(), 1);
    }

    #[test]
    fn flags_replacement_and_description_parses() {
        for src in [
            "if fix.new_text.starts_with(\"package require\") {}",
            "let p = action.description.as_str().strip_prefix(\"Add \");",
            "if (diagnostic.message.includes(\"'\")) {}",
        ] {
            let typescript = src.contains("includes");
            assert_eq!(flagged_lines(src, typescript).len(), 1, "{src}");
        }
    }

    #[test]
    fn formatting_and_typed_reads_pass() {
        for src in [
            "let shown = d.message.trim().to_owned();",
            "let name = d.unresolved_command().unwrap().name_input();",
            "out.push_str(&d.message);",
            "let m = message.contains(\"x\");",
        ] {
            assert!(flagged_lines(src, false).is_empty(), "{src}");
        }
    }

    #[test]
    fn test_modules_and_waivers_are_exempt() {
        let test_mod =
            "fn f() {}\n#[cfg(test)]\nmod tests {\n    fn g() { d.message.contains(\"x\"); }\n}\n";
        assert!(flagged_lines(test_mod, false).is_empty());
        let waived_src = "// presentation-ok: punctuation only\nif d.message.ends_with('?') {}\n";
        assert!(flagged_lines(waived_src, false).is_empty());
    }
}
