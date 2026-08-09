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

//! Input-document resolution — reading input documents, combining sources, and
//! the supporting discovery helpers.

use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use tcl_lsp_core::source_decode::{DecodeReport, decode_source, encoding_integrity_diagnostics};
use tcl_lsp_core::source_style::StyleDiagnostic;

/// Source file extensions the CLI accepts — the registry's single list, shared
/// with the LSP server's workspace scan and the VS Code activation glob.
///
/// `test` is the standard `tcltest` suite-file extension — `tcl check
/// path/to/tests/` skipped a project's whole test suite without it, the CLI
/// twin of the workspace-scan gap in issue #923 differential-audit findings
/// idx 10 / idx 27. The CLI's own copy had additionally drifted from the
/// server's by `exp` / `apl` (issue #1242).
use tcl_registry::dialects::TCL_SOURCE_EXTENSIONS as SOURCE_SUFFIXES;

/// Directory names skipped during recursive discovery.
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

/// Errors raised while resolving CLI input. Rendered as `error: {msg}`
/// by the binary, exit code 2.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// A user-facing input/usage problem, message matched to the captured text.
    #[error("{0}")]
    Input(String),
    /// An underlying I/O failure while reading a file or stdin.
    #[error("{0}")]
    Io(String),
}

impl CliError {
    /// Construct an input/usage error from any displayable message.
    pub fn input(msg: impl Into<String>) -> Self {
        CliError::Input(msg.into())
    }
}

/// A resolved input document.
#[derive(Debug, Clone)]
pub struct InputDocument {
    /// Human-readable label: a file path, `<inline:N>`, or `<stdin>`.
    pub label: String,
    /// The document source text.
    pub source: String,
    /// The originating file path, if any.
    pub path: Option<PathBuf>,
    /// What the decoder had to substitute to produce [`Self::source`] from the
    /// bytes on disk (issue #1326).
    ///
    /// [`DecodeReport::is_faithful`] holds for every document read from text
    /// the caller already had — `--source`, stdin — because there were no bytes
    /// to mis-decode; for a file it is the real verdict, and is what lets
    /// [`encoding_diagnostics`] name a byte offset and a malformation class
    /// instead of guessing from the U+FFFDs left behind.
    pub decode: DecodeReport,
}

impl InputDocument {
    /// The analysis dialect for this document.
    ///
    /// An explicitly-passed `--dialect` wins; otherwise the registry's
    /// standard detection runs over the document (a `# tcl-dialect:` /
    /// shebang / content signal, then the file extension), falling back to
    /// `tcl8.6` — the same priority order the LSP server applies, so `tcl
    /// diag` and the editor report the same set for the same file.
    #[must_use]
    pub fn effective_dialect(&self, explicit: Option<&str>) -> String {
        if let Some(d) = explicit {
            return d.to_string();
        }
        tcl_registry::dialects::detect_dialect(&self.source, self.filename(), "tcl8.6").to_string()
    }

    /// The dialect this document's own content or name gives away, or `None`
    /// when nothing does. Same detector as [`Self::effective_dialect`], minus
    /// the `tcl8.6` fallback — so a caller resolving one dialect for several
    /// documents can tell "this file says nothing" from "this file says plain
    /// Tcl".
    #[must_use]
    fn detected_dialect(&self) -> Option<&'static str> {
        let detected = tcl_registry::dialects::detect_dialect(&self.source, self.filename(), "");
        (!detected.is_empty()).then_some(detected)
    }

    /// The originating file name, for the detector's extension tier.
    fn filename(&self) -> Option<&str> {
        self.path.as_deref().and_then(Path::to_str)
    }

    /// The source-text integrity findings for this document — W107 (not valid
    /// UTF-8) and W109 (not UTF-8 text at all).
    ///
    /// Shares its implementation with the LSP server's publish path. The CLI
    /// always has the source bytes. An editor can raise W107 or W109 only while
    /// its Unicode buffer still exactly matches bytes the server read from
    /// disk; otherwise those byte-level checks deliberately abstain. See
    /// [`tcl_lsp_core::source_decode`].
    #[must_use]
    pub fn encoding_diagnostics(&self) -> Vec<StyleDiagnostic> {
        encoding_integrity_diagnostics(&self.source, Some(&self.decode))
    }

    /// Whether analysis of this document should **abstain** — the bytes are not
    /// UTF-8 text, so every finding past the encoding one would be about
    /// characters that are decoding artefacts rather than about the user's
    /// code.
    #[must_use]
    pub fn abstains_on_encoding(&self) -> bool {
        self.decode.requires_abstention()
    }
}

/// The analysis dialect for a verb that works over **all** its input documents
/// at once — the transforms (`format`, `opt`, `minify`), the graph verbs, the
/// explorer, and the compile verbs, which combine their inputs into one source
/// via [`combine_sources`].
///
/// An explicit `--dialect` wins. Otherwise the documents are detected in input
/// order and the first one that gives something away decides the invocation
/// (`probe.irule`, or a `.tcl` file opening `when HTTP_REQUEST {`, selects
/// `f5-irules`); when no document does, the `tcl8.6` fallback applies. This is
/// the same detector and priority order [`InputDocument::effective_dialect`]
/// gives the per-document diagnostics verbs, so `tcl opt` and `tcl diag` agree
/// about what a file is.
#[must_use]
pub fn combined_effective_dialect(documents: &[InputDocument], explicit: Option<&str>) -> String {
    if let Some(d) = explicit {
        return d.to_string();
    }
    documents
        .iter()
        .find_map(InputDocument::detected_dialect)
        .unwrap_or("tcl8.6")
        .to_owned()
}

/// `pkgIndex.tcl` is always accepted; otherwise the extension must be known.
fn is_supported_source_file(path: &Path) -> bool {
    if path.file_name().is_some_and(|n| n == "pkgIndex.tcl") {
        return true;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => {
            let lower = ext.to_ascii_lowercase();
            SOURCE_SUFFIXES.contains(&lower.as_str())
        }
        None => false,
    }
}

/// Discover supported source files under `directory`, sorted, honouring the
/// skip-directory set and the leading-dot rule.
fn iter_directory_sources(directory: &Path, recursive: bool) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    if recursive {
        walk_recursive(directory, &mut files)?;
    } else {
        let mut entries: Vec<PathBuf> = read_dir_sorted(directory)?;
        entries.sort();
        for path in entries {
            if path.is_file() && is_supported_source_file(&path) {
                files.push(canonical(&path));
            }
        }
    }
    Ok(files)
}

/// Recursive directory walk, visiting child directories in sorted order and
/// files in sorted order.
fn walk_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), CliError> {
    let entries = read_dir_sorted(dir)?;
    let mut subdirs: Vec<PathBuf> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();
    for path in entries {
        if path.is_dir() {
            let keep = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !SKIP_DIRECTORY_NAMES.contains(&n) && !n.starts_with('.'));
            if keep {
                subdirs.push(path);
            }
        } else if path.is_file() && is_supported_source_file(&path) {
            files.push(canonical(&path));
        }
    }
    files.sort();
    out.extend(files);
    subdirs.sort();
    for sub in subdirs {
        walk_recursive(&sub, out)?;
    }
    Ok(())
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>, CliError> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| CliError::Io(format!("failed to read directory {}: {e}", dir.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    Ok(entries)
}

/// Resolve to an absolute path, falling back to the input on failure. Does
/// not require the path to exist, since the caller has already checked.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Resolve CLI inputs into ordered [`InputDocument`]s.
///
/// Resolves inputs in order: inline `--source` snippets come first
/// (labelled `<inline:N>`), then files and directory-discovered files
/// (deduplicated, UTF-8 with lossy replacement). If nothing resolves and stdin
/// is not a TTY, stdin is read as `<stdin>`; otherwise an error is returned.
///
/// Package-name inputs (a bare token that is not an existing path) are not
/// supported — they require the `tclpkg` resolver — and produce a clear error
/// rather than being silently dropped.
pub fn read_input_documents(
    inputs: &[PathBuf],
    inline_sources: &[String],
    recursive: bool,
) -> Result<Vec<InputDocument>, CliError> {
    let mut ordered_files: Vec<PathBuf> = Vec::new();
    let mut package_names: Vec<String> = Vec::new();

    for raw in inputs {
        let path = expand_user(raw);
        if !path.exists() {
            package_names.push(raw.display().to_string());
            continue;
        }
        if path.is_file() {
            if !is_supported_source_file(&path) {
                return Err(CliError::input(format!(
                    "unsupported source file: {} (expected Tcl/iRules file extensions)",
                    path.display()
                )));
            }
            ordered_files.push(canonical(&path));
        } else if path.is_dir() {
            let discovered = iter_directory_sources(&canonical(&path), recursive)?;
            if discovered.is_empty() {
                return Err(CliError::input(format!(
                    "directory has no supported Tcl source files: {}",
                    path.display()
                )));
            }
            ordered_files.extend(discovered);
        } else {
            return Err(CliError::input(format!(
                "unsupported input path type: {}",
                path.display()
            )));
        }
    }

    if !package_names.is_empty() {
        return Err(CliError::input(format!(
            "package resolution is not yet implemented: {}",
            package_names.join(", ")
        )));
    }

    let mut documents: Vec<InputDocument> = Vec::new();
    for (index, source_text) in inline_sources.iter().enumerate() {
        documents.push(InputDocument {
            label: format!("<inline:{}>", index + 1),
            source: source_text.clone(),
            path: None,
            // `--source` text arrived as a `String` already: there were never
            // any bytes for us to mis-decode.
            decode: DecodeReport::default(),
        });
    }

    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for file_path in ordered_files {
        if !seen.insert(file_path.clone()) {
            continue;
        }
        let bytes = std::fs::read(&file_path)
            .map_err(|e| CliError::input(format!("failed to read {}: {e}", file_path.display())))?;
        // The one byte -> text boundary for Tcl source: still a lossy decode
        // (so a broken file is analysed rather than refused), but no longer a
        // silent one — `decode` carries exactly what was substituted, and
        // `encoding_diagnostics` turns it into a real finding. Issue #1326.
        let (source, decode) = decode_source(&bytes);
        documents.push(InputDocument {
            label: file_path.display().to_string(),
            source,
            path: Some(file_path),
            decode,
        });
    }

    if documents.is_empty() && !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| CliError::Io(format!("failed to read stdin: {e}")))?;
        documents.push(InputDocument {
            label: "<stdin>".to_owned(),
            source: buf,
            path: None,
            // `read_to_string` has already refused non-UTF-8 stdin with an
            // `Io` error above, so anything that reaches here is faithful.
            decode: DecodeReport::default(),
        });
    }

    if documents.is_empty() {
        return Err(CliError::input(
            "no input provided; pass files/directories/packages, --source, or pipe stdin",
        ));
    }

    Ok(documents)
}

/// Combine documents into one source string: each doc's trailing newlines are
/// stripped and the chunks are joined with a blank line (mirrors
/// `_combine_sources`).
#[must_use]
pub fn combine_sources(documents: &[InputDocument]) -> String {
    documents
        .iter()
        .map(|d| d.source.trim_end_matches('\n'))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Expand a leading `~` to the user's home directory (best-effort).
fn expand_user(path: &Path) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path.to_path_buf();
    };
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(rest);
        }
    } else if s == "~"
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home);
    }
    path.to_path_buf()
}
