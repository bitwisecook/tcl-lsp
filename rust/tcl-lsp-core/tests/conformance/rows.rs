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

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_lexer::LineIndex;

use crate::engines::{Interpreters, Outcome};

/// A probe fixture: its file and its path below the probe root.
#[derive(Clone)]
pub struct Fixture {
    pub path: PathBuf,
    /// Directory below the probe root, `/`-separated (`namespaces/nav`).
    pub area: String,
    pub file: String,
}

impl Fixture {
    pub fn name(&self) -> String {
        format!("{}/{}", self.area, self.file)
    }
}

pub struct Diag {
    pub code: String,
    pub line: usize,
    pub text: String,
    pub message: String,
}

/// The editor's answers at one `;# CALL` or `;# READ` mark.
pub struct EditorAnswers {
    pub word: String,
    pub at: String,
    /// Definition targets as (line, `LlineCcolumn`, declaration text).
    pub definition: Vec<(u32, String, String)>,
    pub references: Vec<String>,
    pub rename: Vec<String>,
    pub hover: String,
    /// Diagnostic codes on the marked line.
    pub diagnostics: Vec<String>,
    /// Completion labels at a `;# COMPLETE` mark, in the order offered.
    pub completion: Option<Vec<String>>,
}

pub struct Dialect {
    pub name: &'static str,
    pub diagnostics: Vec<Diag>,
    /// Rewrite codes the folder proposes, in source order.
    pub folds: Vec<String>,
    /// `None` when the folder proposes no rewrite; otherwise whether C Tcl
    /// of the dialect's release runs the rewritten file the same way.
    pub folded_execution: Option<String>,
    pub editor: Vec<EditorAnswers>,
}

/// Every column of one fixture row.
pub struct Row {
    pub engines: Vec<(String, Outcome)>,
    pub dialects: Vec<Dialect>,
}

fn dialects_for(area: &str) -> &'static [&'static str] {
    if area == "jim" {
        &["tcl8.6", "tcl9.0", "jimtcl"]
    } else {
        &["tcl8.6", "tcl9.0"]
    }
}

const TIMING_MARK: &str = "# conformance: timing-sensitive.";

pub fn compute(fixture: &Fixture, interpreters: &Interpreters, root: &Path) -> Row {
    let bytes =
        std::fs::read(&fixture.path).unwrap_or_else(|error| panic!("{}: {error}", fixture.name()));
    // Encoding fixtures carry bytes that are not UTF-8; the analysis columns
    // see the editor's lossy view and the folder is not executed on them.
    let exact = std::str::from_utf8(&bytes).is_ok();
    let source = String::from_utf8_lossy(&bytes).into_owned();
    // A fixture that documents a timing race records only its exit status.
    let timing = source.starts_with(TIMING_MARK);
    let engines = interpreters
        .all
        .iter()
        .map(|interpreter| {
            let mut outcome = run_copy(fixture, &bytes, |file| interpreter.run(file, root));
            if timing {
                "(timing-sensitive output not compared)".clone_into(&mut outcome.output);
            }
            (interpreter.column.clone(), outcome)
        })
        .collect();
    let dialects = dialects_for(&fixture.area)
        .iter()
        .map(|&name| dialect_columns(fixture, &source, exact, name, interpreters, root))
        .collect();
    Row { engines, dialects }
}

fn line_of(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset.min(source.len())]
        .split(|&byte| byte == b'\n')
        .count()
}

fn analyser_diagnostics(source: &str, name: &'static str) -> (AnalysisResult, Vec<Diag>) {
    let analysis = Analyser::new().analyse(source, name);
    let mut diagnostics: Vec<Diag> = analysis
        .diagnostics
        .iter()
        .map(|diag| {
            let range = diag.span.as_range();
            Diag {
                code: diag.code.as_str().to_owned(),
                line: line_of(source, range.start),
                text: source
                    .get(range.start..range.end.min(source.len()))
                    .unwrap_or_default()
                    .to_owned(),
                message: diag.message.clone(),
            }
        })
        .collect();
    diagnostics.sort_by(|a, b| (a.line, &a.code, &a.message).cmp(&(b.line, &b.code, &b.message)));
    (analysis, diagnostics)
}

fn dialect_columns(
    fixture: &Fixture,
    source: &str,
    exact: bool,
    name: &'static str,
    interpreters: &Interpreters,
    root: &Path,
) -> Dialect {
    let (analysis, diagnostics) = analyser_diagnostics(source, name);

    // Jim has an analyser environment but no dialect profile for the folder
    // or the editor features.
    let Some(profile) = DialectProfile::find(name) else {
        return Dialect {
            name,
            diagnostics,
            folds: Vec::new(),
            folded_execution: None,
            editor: Vec::new(),
        };
    };
    let environment = tcl_registry::model::ingress::resolve_environment(name);
    let context = environment.default_context_registry();
    let proposals =
        tcl_compiler::optimiser::optimise_with_dialect(source, context.commands(), Some(profile));
    // Hints carry no replacement and are never applied; the executed file
    // takes every applicable rewrite, as `minify` does.
    let folds = proposals
        .iter()
        .map(|rewrite| {
            let code = rewrite.code.as_str();
            if rewrite.hint_only {
                format!("hint:{code}")
            } else {
                code.to_owned()
            }
        })
        .collect::<Vec<_>>();
    let optimisations: Vec<_> = proposals
        .iter()
        .filter(|rewrite| !rewrite.hint_only)
        .collect();
    let release = TclVersion::from_version_string(name.trim_start_matches("tcl"));
    let invalid = optimisations.iter().find(|rewrite| {
        let range = rewrite.span.as_range();
        range.end > source.len()
            || !source.is_char_boundary(range.start)
            || !source.is_char_boundary(range.end)
    });
    let mut ranges: Vec<_> = optimisations
        .iter()
        .map(|rewrite| rewrite.span.as_range())
        .collect();
    ranges.sort_by_key(|range| (range.start, range.end));
    let overlap = ranges.windows(2).find(|pair| pair[1].start < pair[0].end);
    let folded_execution = match (
        optimisations.is_empty() || !exact,
        release,
        invalid,
        overlap,
    ) {
        (true, _, _, _) | (_, None, _, _) => None,
        (false, Some(_), Some(rewrite), _) => Some(format!(
            "invalid rewrite span {}..{} for {} bytes",
            rewrite.span.as_range().start,
            rewrite.span.as_range().end,
            source.len()
        )),
        (false, Some(_), None, Some(pair)) => Some(format!(
            "overlapping rewrites {}..{} and {}..{}",
            pair[0].start, pair[0].end, pair[1].start, pair[1].end
        )),
        (false, Some(release), None, None) => {
            let mut folded = source.to_owned();
            let mut ordered = optimisations.clone();
            ordered.sort_by_key(|rewrite| std::cmp::Reverse(rewrite.span.as_range().start));
            for rewrite in ordered {
                folded.replace_range(rewrite.span.as_range(), &rewrite.replacement);
            }
            let tclsh = interpreters.tclsh(release);
            let before = run_copy(fixture, source.as_bytes(), |file| tclsh.run(file, root));
            let after = run_copy(fixture, folded.as_bytes(), |file| tclsh.run(file, root));
            Some(if before == after {
                "same".to_owned()
            } else {
                format!(
                    "differs: before [{}] {} | after [{}] {}",
                    before.exit, before.output, after.exit, after.output
                )
            })
        }
    };
    let editor = editor_answers(source, profile, &analysis, context.commands());
    Dialect {
        name,
        diagnostics,
        folds,
        folded_execution,
        editor,
    }
}

static COPIES: AtomicUsize = AtomicUsize::new(0);

/// Run `text` under the fixture's own name in a fresh copy of the fixture's
/// directory, so `info script` and relative `source` agree between runs and
/// files a fixture writes never reach the probe tree or the next run.
fn run_copy(fixture: &Fixture, text: &[u8], run: impl FnOnce(&Path) -> Outcome) -> Outcome {
    let directory = std::env::temp_dir().join(format!(
        "tcl-conformance-{}-{}",
        std::process::id(),
        COPIES.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory");
    copy_tree(
        fixture.path.parent().expect("fixture directory"),
        &directory,
    );
    let file = directory.join(&fixture.file);
    std::fs::write(&file, text).expect("scratch fixture");
    let outcome = run(&file);
    let _ = std::fs::remove_dir_all(&directory);
    outcome
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).expect("fixture directory listing") {
        let entry = entry.expect("directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("entry type").is_dir() {
            std::fs::create_dir_all(&target).expect("scratch subdirectory");
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("scratch copy");
        }
    }
}

struct Mark {
    word: String,
    cursor: usize,
    /// A `;# COMPLETE` mark asks for completion as well.
    complete: bool,
}

/// `;# CALL` marks the last word before it, or the named word
/// (`;# CALL get_a`); `;# READ` marks the variable of the last `$` before it;
/// `;# COMPLETE word` marks the end of `word` on the next line holding it, so
/// a mark can sit outside a braced body (a `switch` arm) it must not change.
fn marks(source: &str) -> Vec<Mark> {
    let mut found = Vec::new();
    let mut line_start = 0;
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    for (index, line) in lines.iter().enumerate() {
        if let Some(at) = line.find(";# COMPLETE") {
            let named = line[at + ";# COMPLETE".len()..].split_whitespace().next();
            let mut start = line_start + line.len();
            for next in &lines[index + 1..] {
                if let Some(word) = named
                    && let Some(column) = next.find(word)
                {
                    found.push(Mark {
                        word: word.to_owned(),
                        cursor: start + column + word.len(),
                        complete: true,
                    });
                    break;
                }
                start += next.len();
            }
        }
        for (kind, tag) in [("CALL", ";# CALL"), ("READ", ";# READ")] {
            let Some(at) = line.find(tag) else { continue };
            let before = &line[..at];
            let named = line[at + tag.len()..].split_whitespace().next();
            let last_word = || {
                let trimmed = before.trim_end();
                trimmed
                    .rfind(|c: char| c.is_whitespace() || c == '[' || c == '{')
                    .map_or(Some(0), |space| Some(space + 1))
            };
            let cursor = if kind == "READ" {
                before
                    .rfind('$')
                    .map(|dollar| dollar + 1 + usize::from(before[dollar + 1..].starts_with('{')))
                    .or_else(last_word)
            } else if let Some(word) = named {
                before.rfind(word)
            } else {
                last_word()
            };
            let Some(cursor) = cursor else { continue };
            let word = if kind == "READ" {
                before[cursor..].trim_end().to_owned()
            } else {
                before[cursor..]
                    .split(|c: char| c.is_whitespace() || "[]{}();\"".contains(c))
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            };
            found.push(Mark {
                word,
                cursor: line_start + cursor,
                complete: false,
            });
        }
        line_start += line.len();
    }
    found
}

fn position(range: &tcl_lsp_core::definition::LspRange) -> String {
    format!("L{}c{}", range.start_line + 1, range.start_character)
}

/// How many completion labels a `;# COMPLETE` row records.
const COMPLETION_LABELS: usize = 12;

fn editor_answers(
    source: &str,
    profile: &'static DialectProfile,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<EditorAnswers> {
    let index = LineIndex::new(source);
    marks(source)
        .into_iter()
        .map(|mark| {
            let offset = u32::try_from(mark.cursor).expect("fixture offset fits u32");
            let pos = index.position_at_utf16(offset, source);
            let (line, character) = (pos.line, pos.character.get());
            let definition =
                tcl_lsp_core::definition::definition(source, line, character, analysis)
                    .iter()
                    .map(|range| {
                        let text = source
                            .lines()
                            .nth(range.start_line as usize)
                            .unwrap_or_default();
                        (
                            range.start_line + 1,
                            position(range),
                            text.trim().chars().take(40).collect::<String>(),
                        )
                    })
                    .collect();
            let references = tcl_lsp_core::references::references(
                source, profile, line, character, analysis, true,
            )
            .iter()
            .map(position)
            .collect();
            let mut rename: Vec<String> = tcl_lsp_core::rename::rename(
                source,
                profile,
                line,
                character,
                "renamed",
                analysis,
                Some(registry),
            )
            .iter()
            .map(|edit| position(&edit.range))
            .collect();
            rename.sort();
            let hover =
                tcl_lsp_core::hover::hover(source, line, character, analysis, Some(registry))
                    .and_then(|hover| {
                        hover
                            .value
                            .lines()
                            .find(|text| !text.is_empty() && !text.starts_with("```"))
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "-".to_owned());
            let diagnostics = analysis
                .diagnostics
                .iter()
                .filter(|diag| index.position_at_utf16(diag.span.start(), source).line == line)
                .map(|diag| diag.code.as_str().to_owned())
                .collect();
            let completion = mark.complete.then(|| {
                tcl_lsp_core::completion::completions(
                    source,
                    line,
                    character,
                    analysis,
                    Some(registry),
                    None,
                    profile,
                )
                .into_iter()
                .take(COMPLETION_LABELS)
                .map(|item| item.label)
                .collect()
            });
            EditorAnswers {
                completion,
                at: source[mark.cursor..]
                    .chars()
                    .take(mark.word.chars().count() + 1)
                    .collect(),
                word: mark.word,
                definition,
                references,
                rename,
                hover,
                diagnostics,
            }
        })
        .collect()
}

/// Every fixture below `root`, in path order, excluding the definitions
/// matrix (its own gate) and generated output directories.
pub fn fixtures(root: &Path) -> Vec<Fixture> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("probe directory") {
            let path = entry.expect("directory entry").path();
            let relative = path.strip_prefix(root).expect("below root");
            let first = relative
                .components()
                .next()
                .and_then(|part| part.as_os_str().to_str())
                .unwrap_or_default();
            if matches!(first, "definitions" | "tcl-areas-head") {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "tcl") {
                let area = relative
                    .parent()
                    .map_or_default(|parent| parent.to_string_lossy().replace('\\', "/"));
                let file = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("UTF-8 fixture name")
                    .to_owned();
                found.push(Fixture { path, area, file });
            }
        }
    }
    found.sort_by(|a, b| (&a.area, &a.file).cmp(&(&b.area, &b.file)));
    found
}
