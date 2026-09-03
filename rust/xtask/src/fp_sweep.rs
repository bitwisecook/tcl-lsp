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

//! `cargo xtask fp-sweep` — the false-positive audit harness (issue #1316).
//!
//! It implements the method documented in `docs/design/compiler/fp-sweep.md`:
//! dump every firing of a code across the corpus, dialect-aware, grouped by
//! site shape.
//!
//! - **Dialect-aware**: each corpus file's analysis dialect is resolved with
//!   [`tcl_cli_support::InputDocument::effective_dialect`] — the same
//!   detector (`# tcl-dialect:` / content signal / extension, falling back to
//!   `tcl8.6`) the `tcl` CLI and the LSP server use — so a version-gated
//!   command in a Tcl-9-only file does not produce a phantom W002/W004 the
//!   way a fixed-dialect sweep would.
//! - **Every code, one pass**: both diagnostic sources the editor publishes —
//!   [`Analyser::analyse`]'s W/E/H-series checks and
//!   [`run_all_checks`]'s O/S/T-series compiler-checks pass — are run per
//!   file, mirroring `tcl diag`'s `collect_rows` (`rust/tcl-cli/src/commands/diag.rs`)
//!   minus its optimisation-code filter, since an FP sweep needs the O-series
//!   firings `tcl diag` deliberately drops.
//! - **Grouped by site shape**: firings for a code are bucketed by their
//!   normalised message (digit runs and single-quoted identifiers replaced
//!   with a placeholder) so repeated instances of the same pattern collapse
//!   into one row with a count, highest-volume shape first — the triage
//!   workflow `fp-sweep.md` describes.
//!
//! Corpus discovery accepts the normal Tcl-family extensions plus two
//! corpus-only publication formats common in the #1181 iRules sources:
//!
//! - a `.txt` file containing a top-level `when EVENT ... {` handler is swept
//!   as one iRules document;
//! - a reStructuredText `.rst` `code` / `code-block` directive is extracted
//!   when its indented body contains that same signature. Its findings retain
//!   the source document's original line numbers.
//!
//! The strong event-handler signal keeps arbitrary prose and console blocks
//! out of the analyser. Normal source files are still decoded by
//! [`tcl_cli_support::read_input_documents`], the same reader as `tcl diag` /
//! `tcl opt`. The seven `.tmsh` files in the pinned public #1181 corpus are
//! BIG-IP configuration/data-group or iCall artefacts, not iRules (none has a
//! `when EVENT` handler), so they remain outside this iRules diagnostic sweep.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tcl_cli_support::{InputDocument, read_input_documents};
use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::optimiser::optimise_unit;
use tcl_core_types::DiagCode;
use tcl_lexer::LineIndex;
use tcl_lsp_core::source_decode::{DecodeReport, decode_source};
use tcl_lsp_core::source_style::{DEFAULT_LINE_ENDING, DEFAULT_LINE_LENGTH, style_diagnostics};
use tcl_registry::dialects::TCL_SOURCE_EXTENSIONS;

/// Directory names skipped by the corpus walk. Keep this aligned with the
/// normal CLI input walk: generated/build trees and VCS metadata are neither
/// user source nor useful third-party audit material.
const SKIP_DIRECTORY_NAMES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".venv",
    "__pycache__",
    "node_modules",
    "build",
    "dist",
];

/// A document entering the sweep, plus provenance needed only by corpus
/// extraction. Native documents use no override and a zero offset.
struct SweepDocument {
    input: InputDocument,
    line_offset: u32,
    dialect_override: Option<&'static str>,
}

#[derive(Default)]
struct CorpusDocuments {
    documents: Vec<SweepDocument>,
    text_files: usize,
    rst_files: usize,
    rst_blocks: usize,
}

impl CorpusDocuments {
    fn source_files(&self) -> usize {
        self.documents.len() - self.rst_blocks + self.rst_files
    }
}

/// One extracted reStructuredText literal block. `line_offset` is zero-based,
/// so a diagnostic at extracted line 1 maps to original line
/// `line_offset + 1`.
#[derive(Debug, PartialEq, Eq)]
struct RstBlock {
    source: String,
    line_offset: u32,
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn extension_is(path: &Path, wanted: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(wanted))
}

fn is_native_source(path: &Path) -> bool {
    if path.file_name().is_some_and(|name| name == "pkgIndex.tcl") {
        return true;
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            TCL_SOURCE_EXTENSIONS
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        })
}

fn is_irules_when_clause_tail(tail: &str) -> bool {
    let mut words = tail.split_whitespace();
    while let Some(word) = words.next() {
        match word {
            "priority" => {
                if !words
                    .next()
                    .is_some_and(|value| value.chars().all(|c| c.is_ascii_digit()))
                {
                    return false;
                }
            }
            "timing" => {
                if !words
                    .next()
                    .is_some_and(|value| value.chars().all(|c| c.is_ascii_alphabetic()))
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// The deliberately strong signature used to opt non-source extensions into
/// the iRules sweep: a line-leading `when`, an all-caps event identifier, and
/// an opening brace later on the same line. This accepts real optional clauses
/// (`priority 500`, `timing on`) without teaching the harness their grammar.
fn has_irules_event(source: &str) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    lines.iter().enumerate().any(|(index, line)| {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("when") else {
            return false;
        };
        if !rest.starts_with(char::is_whitespace) {
            return false;
        }
        let rest = rest.trim_start();
        let event_len = rest
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .map(char::len_utf8)
            .sum::<usize>();
        event_len >= 3
            && rest[..event_len]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
            && (rest[event_len..].contains('{')
                // The BIG-IP man-page schema also carries canonical examples
                // with the opening brace on the following line (for example,
                // `when ASM_REQUEST_VIOLATION\n{`). Accept that Tcl layout,
                // but only when the next non-empty line begins with the brace.
                || (is_irules_when_clause_tail(rest[event_len..].trim())
                    && lines[index + 1..]
                        .iter()
                        .map(|next| next.trim_start())
                        .find(|next| !next.is_empty())
                        .is_some_and(|next| next.starts_with('{'))))
    })
}

fn indentation(line: &str) -> usize {
    line.chars().take_while(|c| matches!(c, ' ' | '\t')).count()
}

fn is_rst_code_directive(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix("..") else {
        return false;
    };
    let rest = rest.trim_start();
    let language = if let Some(rest) = rest.strip_prefix("code-block::") {
        rest.trim()
    } else if let Some(rest) = rest.strip_prefix("code::") {
        rest.trim()
    } else {
        return false;
    };

    // An unlabelled literal block is common in class1 of the agility-labs
    // corpus. Explicit Tcl/iRules labels are accepted; console, JavaScript,
    // JSON, and other teaching snippets are not.
    language.is_empty()
        || language.eq_ignore_ascii_case("tcl")
        || language.eq_ignore_ascii_case("irule")
        || language.eq_ignore_ascii_case("irules")
}

/// Extract iRules-bearing `code` / `code-block` directives from an `.rst`
/// document. Indentation is retained so reported columns also refer to the
/// original document; Tcl accepts the leading whitespace at top level.
fn extract_rst_irules(source: &str) -> Vec<RstBlock> {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut blocks = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if !is_rst_code_directive(lines[index].trim_end_matches(['\r', '\n'])) {
            index += 1;
            continue;
        }

        let directive_indent = indentation(lines[index]);
        let mut start = index + 1;

        // Directive options (`:linenos:`, `:emphasize-lines:`) and blank
        // separators precede the literal body. They are RST metadata, not Tcl.
        while start < lines.len() {
            let line = lines[start].trim_end_matches(['\r', '\n']);
            if line.trim().is_empty()
                || (indentation(line) > directive_indent && line.trim_start().starts_with(':'))
            {
                start += 1;
            } else {
                break;
            }
        }

        if start >= lines.len() || indentation(lines[start]) <= directive_indent {
            index += 1;
            continue;
        }

        let mut end = start;
        while end < lines.len() {
            let line = lines[end].trim_end_matches(['\r', '\n']);
            if !line.trim().is_empty() && indentation(line) <= directive_indent {
                break;
            }
            end += 1;
        }

        let block_source = lines[start..end].concat();
        if has_irules_event(&block_source) {
            blocks.push(RstBlock {
                source: block_source,
                line_offset: u32::try_from(start).unwrap_or(u32::MAX),
            });
        }
        index = end.max(index + 1);
    }

    blocks
}

fn walk_corpus(
    directory: &Path,
    native: &mut BTreeSet<PathBuf>,
    embedded: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let entries = std::fs::read_dir(directory)
        .with_context(|| format!("reading corpus directory {}", directory.display()))?;
    for entry in entries {
        let path = entry
            .with_context(|| format!("reading corpus directory {}", directory.display()))?
            .path();
        if path.is_dir() {
            let keep = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    !name.starts_with('.') && !SKIP_DIRECTORY_NAMES.contains(&name)
                });
            if keep {
                walk_corpus(&path, native, embedded)?;
            }
        } else if path.is_file() {
            if is_native_source(&path) {
                native.insert(canonical(&path));
            } else if extension_is(&path, "txt") || extension_is(&path, "rst") {
                embedded.insert(canonical(&path));
            }
        }
    }
    Ok(())
}

fn discover_corpus(inputs: &[PathBuf]) -> Result<(BTreeSet<PathBuf>, BTreeSet<PathBuf>)> {
    let mut native = BTreeSet::new();
    let mut embedded = BTreeSet::new();
    for input in inputs {
        if !input.exists() {
            bail!("corpus path does not exist: {}", input.display());
        }
        if input.is_dir() {
            walk_corpus(&canonical(input), &mut native, &mut embedded)?;
        } else if input.is_file() && is_native_source(input) {
            native.insert(canonical(input));
        } else if input.is_file() && (extension_is(input, "txt") || extension_is(input, "rst")) {
            embedded.insert(canonical(input));
        } else {
            bail!(
                "unsupported corpus file: {} (expected Tcl/iRules source, .txt, or .rst)",
                input.display()
            );
        }
    }
    Ok((native, embedded))
}

fn read_corpus_documents(inputs: &[PathBuf]) -> Result<CorpusDocuments> {
    let (native_paths, embedded_paths) = discover_corpus(inputs)?;
    let mut corpus = CorpusDocuments::default();

    if !native_paths.is_empty() {
        let native_inputs: Vec<PathBuf> = native_paths.into_iter().collect();
        let documents = read_input_documents(&native_inputs, &[], false)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("reading native corpus from {inputs:?}"))?;
        corpus
            .documents
            .extend(documents.into_iter().map(|input| SweepDocument {
                input,
                line_offset: 0,
                dialect_override: None,
            }));
    }

    for path in embedded_paths {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("reading embedded corpus file {}", path.display()))?;
        let (source, decode) = decode_source(&bytes);
        if extension_is(&path, "txt") {
            if has_irules_event(&source) {
                corpus.text_files += 1;
                corpus.documents.push(SweepDocument {
                    input: InputDocument {
                        label: path.display().to_string(),
                        source,
                        path: Some(path),
                        decode,
                    },
                    line_offset: 0,
                    dialect_override: Some("f5-irules"),
                });
            }
            continue;
        }

        let blocks = extract_rst_irules(&source);
        if blocks.is_empty() {
            continue;
        }
        corpus.rst_files += 1;
        corpus.rst_blocks += blocks.len();
        for (block_index, block) in blocks.into_iter().enumerate() {
            corpus.documents.push(SweepDocument {
                input: InputDocument {
                    label: format!("{}#irule-{}", path.display(), block_index + 1),
                    source: block.source,
                    path: Some(path.clone()),
                    // The containing file was decoded above. Extracted text no
                    // longer has a byte-for-byte range against the whole-file
                    // decode report, so byte-integrity diagnostics abstain.
                    decode: DecodeReport::default(),
                },
                line_offset: block.line_offset,
                dialect_override: Some("f5-irules"),
            });
        }
    }

    if corpus.documents.is_empty() {
        bail!("corpus contains no Tcl or iRules documents: {inputs:?}");
    }
    Ok(corpus)
}

/// One firing of a swept code.
struct Firing {
    code: DiagCode,
    file: String,
    dialect: String,
    line: u32,
    column: u32,
    message: String,
}

/// Replace every run of ASCII digits and every single-quoted token with a
/// placeholder, so e.g. `"proc 'foo' shadows builtin 'set'"` and `"proc
/// 'bar' shadows builtin 'clock'"` collapse into one site-shape bucket
/// (`"proc '…' shadows builtin '…'"`) instead of each getting their own row.
fn shape_key(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            out.push('…');
            while chars.peek().is_some_and(char::is_ascii_digit) {
                chars.next();
            }
        } else if c == '\'' {
            out.push('\'');
            out.push('…');
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == '\'' {
                    closed = true;
                    break;
                }
            }
            if closed {
                out.push('\'');
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Collect every firing of a code in `wanted` across `doc`, running every
/// diagnostic source the editor publishes for one document:
///
/// 1. [`Analyser::analyse`] — the W/E/H-series syntactic/semantic checks,
///    built over a default-config unit, mirroring `tcl diag`'s
///    `collect_rows`.
/// 2. [`run_all_checks`] + [`optimise_unit`] — the O/S/T-series
///    compiler-checks and optimiser passes, over one dialect-configured,
///    interprocedural-summarised unit — mirroring
///    `tcl_lsp_db::compiler_check_diagnostics_uncached`, the documented
///    "no salsa input" fallback that is byte-identical to the live server's
///    build. `tcl diag` only runs (1); an FP sweep needs the O-series
///    firings it deliberately drops, so this reproduces the fuller build
///    rather than diag.rs's simplified (non-interprocedural) one.
/// 3. [`style_diagnostics`] — the pure-text W111/W112/W115/W118 checks,
///    which read raw source and are not part of either compiler pass.
fn sweep_document(doc: &SweepDocument, wanted: &[DiagCode], out: &mut Vec<Firing>) {
    // The override is a dialect *name*, so it resolves through the one
    // ingress seam, in the `find`-shaped validator form: an override that
    // names no environment falls through to the document's own effective
    // dialect rather than to the lenient sink.
    let profile = doc
        .dialect_override
        .and_then(crate::environment::known_profile_for_dialect)
        .unwrap_or_else(|| doc.input.effective_dialect(None));
    let dialect = profile.name;
    let registry = crate::environment::store_for_dialect(dialect);
    let line_index = LineIndex::new(&doc.input.source);
    let dialect_opt = Some(profile);

    let mut push = |code: DiagCode, span: tcl_lexer::Span, message: String| {
        if !wanted.contains(&code) {
            return;
        }
        let pos = line_index.position_at_utf16(span.start(), &doc.input.source);
        out.push(Firing {
            code,
            file: doc.input.label.clone(),
            dialect: dialect.to_owned(),
            line: pos.line + 1 + doc.line_offset,
            column: pos.character.get() + 1,
            message,
        });
    };

    // (1) Analyser tail. The document's own environment grammar — the same
    // config every other `cu_override` host builds under, and the same one the
    // checks pass below uses (redesign §11.4 row E1: all four hosts used to
    // agree on `LexerConfig::default()`, which was wrong for every non-9.x
    // dialect).
    let analysis_cu = Arc::new(CompilationUnit::build_with_options(
        &doc.input.source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(
                tcl_registry::model::resolve_environment(dialect).grammar(),
            ),
            dialect: tcl_lsp_core::optional_profile_for_dialect(dialect),
            external_call_sites: None,
        },
    ));
    let mut analyser =
        Analyser::new().with_file_path(doc.input.path.as_ref().map(|p| p.display().to_string()));
    analyser.set_cu_override(Arc::clone(&analysis_cu));
    let result = analyser.analyse(&doc.input.source, dialect);
    for d in &result.diagnostics {
        push(d.code, d.span, d.message.clone());
    }

    // (2) Compiler-checks + optimiser, over one interprocedural-summarised,
    // dialect-configured unit.
    let checks_cu = CompilationUnit::build_with_options(
        &doc.input.source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(
                tcl_registry::model::resolve_environment(dialect).grammar(),
            ),
            dialect: tcl_lsp_core::optional_profile_for_dialect(dialect),
            external_call_sites: None,
        },
    )
    .with_interprocedural(registry, dialect_opt);
    for d in run_all_checks(&checks_cu, registry, dialect_opt) {
        push(d.code, d.span, d.message);
    }
    for o in optimise_unit(&checks_cu, registry, dialect_opt) {
        push(o.code, o.span, o.message);
    }

    // (3) Pure-text checks — the style lints plus the byte-backed W107 / W109
    // encoding-integrity set. W305 was already collected from the canonical
    // analyser producer in (1). No suppression / user-disabled set, since the
    // sweep wants every firing regardless of what a hypothetical editor config
    // would silence. Native documents carry the byte-level decode report and
    // therefore produce encoding findings at full precision (issue #1326).
    // Extracted RST blocks use a faithful empty report because their offsets no
    // longer refer to the containing file's byte stream.
    let no_disabled: std::collections::HashSet<String> = std::collections::HashSet::new();
    let no_suppressed: std::collections::HashMap<i32, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    for d in style_diagnostics(
        &doc.input.source,
        DEFAULT_LINE_LENGTH,
        DEFAULT_LINE_ENDING,
        &no_disabled,
        &no_suppressed,
        Some(&doc.input.decode),
        tcl_lsp_core::profile_for_dialect(dialect),
    ) {
        let Ok(code) = DiagCode::from_str(d.code) else {
            continue;
        };
        if !wanted.contains(&code) {
            continue;
        }
        out.push(Firing {
            code,
            file: doc.input.label.clone(),
            dialect: dialect.to_owned(),
            line: d.range.start_line + 1 + doc.line_offset,
            column: d.range.start_character + 1,
            message: d.message,
        });
    }
}

/// Report every firing of `code` in `firings`, grouped by message shape,
/// highest-volume shape first, with up to `examples` sample locations per
/// shape and a per-dialect firing-count breakdown.
fn report_code(code: DiagCode, firings: &[Firing], examples: usize) {
    let mine: Vec<&Firing> = firings.iter().filter(|f| f.code == code).collect();
    println!("== {} — {} firing(s) ==", code.as_str(), mine.len());
    if mine.is_empty() {
        println!("  (no firings in this corpus)\n");
        return;
    }

    let mut by_dialect: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &mine {
        *by_dialect.entry(f.dialect.as_str()).or_insert(0) += 1;
    }
    print!("  by dialect:");
    for (dialect, n) in &by_dialect {
        print!(" {dialect}={n}");
    }
    println!();

    let mut by_shape: BTreeMap<String, Vec<&Firing>> = BTreeMap::new();
    for f in &mine {
        by_shape.entry(shape_key(&f.message)).or_default().push(f);
    }
    let mut shapes: Vec<(&String, &Vec<&Firing>)> = by_shape.iter().collect();
    shapes.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    for (shape, group) in shapes {
        println!("  [{:>5}] {shape}", group.len());
        for f in group.iter().take(examples) {
            println!(
                "           {}:{}:{}: {}",
                f.file, f.line, f.column, f.message
            );
        }
        if group.len() > examples {
            println!("           … {} more", group.len() - examples);
        }
    }
    println!();
}

/// `cargo xtask fp-sweep --code CODE [--code CODE...] --corpus PATH [--corpus PATH...] [--examples N]`
///
/// Prints, per swept code: total firing count, a dialect breakdown, and
/// every distinct message shape ranked by frequency with up to `examples`
/// sample locations — the "dump every firing, group by site/shape" method
/// the checklist specifies.
pub fn run(codes: &[String], corpus: &[PathBuf], examples: usize) -> Result<ExitCode> {
    if codes.is_empty() {
        bail!("pass at least one --code CODE");
    }
    if corpus.is_empty() {
        bail!("pass at least one --corpus PATH (a directory or a file)");
    }

    let wanted: Vec<DiagCode> = codes
        .iter()
        .map(|c| DiagCode::from_str(c).map_err(|_| anyhow::anyhow!("unknown diagnostic code: {c}")))
        .collect::<Result<_>>()?;

    let documents = read_corpus_documents(corpus)?;

    let mut firings: Vec<Firing> = Vec::new();
    for doc in &documents.documents {
        sweep_document(doc, &wanted, &mut firings);
    }

    println!(
        "fp-sweep: {} file(s) scanned, {} code(s) requested, {} firing(s) found",
        documents.source_files(),
        wanted.len(),
        firings.len()
    );
    if documents.text_files != 0 || documents.rst_blocks != 0 {
        println!(
            "          embedded iRules: {} .txt file(s), {} block(s) from {} .rst file(s); {} analysis document(s)",
            documents.text_files,
            documents.rst_blocks,
            documents.rst_files,
            documents.documents.len()
        );
    }
    println!();

    for code in &wanted {
        report_code(*code, &firings, examples);
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irules_event_signal_accepts_clauses_but_rejects_prose_and_lowercase() {
        assert!(has_irules_event("when HTTP_REQUEST {\n  pool web\n}\n"));
        assert!(has_irules_event(
            "  when CLIENT_ACCEPTED priority 500 timing on {\n}\n"
        ));
        assert!(has_irules_event("when ASM_REQUEST_VIOLATION\n{\n}\n"));
        assert!(has_irules_event("when HTTP_REQUEST priority 500\n{\n}\n"));
        assert!(!has_irules_event("Use `when HTTP_REQUEST {` here.\n"));
        assert!(!has_irules_event("when http_request {\n}\n"));
        assert!(!has_irules_event("whenever HTTP_REQUEST {\n}\n"));
    }

    #[test]
    fn rst_extraction_handles_options_generic_blocks_and_original_lines() {
        let rst = r"Heading
-------

.. code-block:: tcl
   :linenos:
   :emphasize-lines: 2

   when HTTP_REQUEST priority 500 {
       HTTP::redirect /
   }

Prose.

.. code::

  when CLIENT_ACCEPTED {
      reject
  }
";

        assert!(is_rst_code_directive(".. code-block:: tcl"));
        assert!(has_irules_event(rst));
        let blocks = extract_rst_irules(rst);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].line_offset, 7);
        assert!(blocks[0].source.starts_with("   when HTTP_REQUEST"));
        assert_eq!(blocks[1].line_offset, 15);
        assert!(blocks[1].source.starts_with("  when CLIENT_ACCEPTED"));
    }

    #[test]
    fn rst_extraction_skips_non_tcl_and_non_irules_code() {
        let rst = r".. code-block:: console

   when HTTP_REQUEST {
   }

.. code-block:: tcl

   set ordinary_tcl 1

.. code-block:: javascript

   when HTTP_RESPONSE {
   }
";
        assert!(extract_rst_irules(rst).is_empty());
    }

    #[test]
    fn nested_rst_directive_keeps_its_body() {
        let rst = r"#. Step

   .. code-block:: irules

      when RULE_INIT {
          set static::enabled 1
      }

   Continue the step.
";
        assert!(is_rst_code_directive("   .. code-block:: irules"));
        assert!(has_irules_event(rst));
        let blocks = extract_rst_irules(rst);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].line_offset, 4);
        assert!(blocks[0].source.contains("when RULE_INIT"));
        assert!(!blocks[0].source.contains("Continue"));
    }
}
