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

//! Output writers — text/highlighted output and the colour/tab resolution
//! helpers.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::input::CliError;

/// Where verb output goes: stdout (`"-"`) or a file.
#[derive(Debug, Clone)]
pub enum OutputTarget {
    /// Standard output.
    Stdout,
    /// A file path.
    File(PathBuf),
}

impl OutputTarget {
    /// Map an optional `--output` path to a target (`None`/`-` → stdout).
    #[must_use]
    pub fn from_arg(output: Option<&Path>) -> Self {
        match output {
            None => OutputTarget::Stdout,
            Some(p) if p.as_os_str() == "-" => OutputTarget::Stdout,
            Some(p) => OutputTarget::File(p.to_path_buf()),
        }
    }

    /// Whether this target is standard output.
    #[must_use]
    pub fn is_stdout(&self) -> bool {
        matches!(self, OutputTarget::Stdout)
    }
}

/// Write text to the target.
///
/// For stdout, a trailing newline is appended when the text is non-empty and
/// does not already end in one. For files, bytes are written verbatim.
pub fn write_text_output(target: &OutputTarget, text: &str) -> Result<(), CliError> {
    match target {
        OutputTarget::Stdout => {
            let mut out = std::io::stdout().lock();
            out.write_all(text.as_bytes())
                .map_err(|e| CliError::Io(format!("failed to write stdout: {e}")))?;
            if !text.is_empty() && !text.ends_with('\n') {
                out.write_all(b"\n")
                    .map_err(|e| CliError::Io(format!("failed to write stdout: {e}")))?;
            }
            Ok(())
        }
        OutputTarget::File(path) => std::fs::write(path, text)
            .map_err(|e| CliError::Io(format!("failed to write {}: {e}", path.display()))),
    }
}

/// Write raw bytes to the target.
///
/// For stdout the payload is written verbatim — no trailing newline, since the
/// bytes are a binary artifact (e.g. a `.wasm` module). For files the bytes are
/// written as-is.
pub fn write_binary_output(target: &OutputTarget, payload: &[u8]) -> Result<(), CliError> {
    match target {
        OutputTarget::Stdout => {
            let mut out = std::io::stdout().lock();
            out.write_all(payload)
                .map_err(|e| CliError::Io(format!("failed to write stdout: {e}")))?;
            out.flush()
                .map_err(|e| CliError::Io(format!("failed to flush stdout: {e}")))
        }
        OutputTarget::File(path) => std::fs::write(path, payload)
            .map_err(|e| CliError::Io(format!("failed to write {}: {e}", path.display()))),
    }
}

/// Write Tcl source, optionally colourised, with faithful tab handling
///
/// When writing to stdout with `tab_width > 0`, tabs are expanded to spaces
/// using tab-stop semantics — and, in the fixed order, this happens
/// *before* ANSI highlighting (so escape codes don't shift the tab stops). The
/// `highlight` verb uses the opposite order; see its handler.
pub fn write_highlighted_output(
    target: &OutputTarget,
    text: &str,
    use_colour: bool,
    tab_width: usize,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Result<(), CliError> {
    let mut rendered = text.to_owned();
    if target.is_stdout() && tab_width > 0 {
        rendered = expand_tabs(&rendered, tab_width);
    }
    if use_colour {
        rendered = crate::highlight::highlight_ansi(&rendered, dialect);
    }
    write_text_output(target, &rendered)
}

/// Expand tab characters to spaces using tab-stop semantics: a tab advances
/// to the next multiple of `tabsize`; column resets on `\n`/`\r`.
#[must_use]
pub fn expand_tabs(text: &str, tabsize: usize) -> String {
    let mut out = String::with_capacity(text.len());
    let mut col = 0usize;
    for ch in text.chars() {
        match ch {
            '\t' => {
                if tabsize > 0 {
                    let spaces = tabsize - (col % tabsize);
                    for _ in 0..spaces {
                        out.push(' ');
                    }
                    col += spaces;
                }
            }
            '\n' | '\r' => {
                out.push(ch);
                col = 0;
            }
            other => {
                out.push(other);
                col += 1;
            }
        }
    }
    out
}

/// Escape non-ASCII characters as `\uXXXX` (with surrogate pairs for astral
/// code points), producing ASCII-only output.
///
/// Apply this to `serde_json::to_string_pretty` output so JSON verbs match the
/// captured golden output byte-for-byte. Safe to apply to a whole JSON document: non-ASCII
/// only ever appears inside already-quoted string values.
#[must_use]
pub fn ensure_ascii(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let cp = c as u32;
            if cp > 0xFFFF {
                let v = cp - 0x1_0000;
                let _ = write!(
                    out,
                    "\\u{:04x}\\u{:04x}",
                    0xD800 + (v >> 10),
                    0xDC00 + (v & 0x3FF)
                );
            } else {
                let _ = write!(out, "\\u{cp:04x}");
            }
        }
    }
    out
}

/// Decide whether ANSI highlighting should be applied (mirrors
/// `_resolve_use_colour`): `--colour` forces on, `--no-colour` forces off,
/// otherwise it is on only when writing to a TTY stdout.
#[must_use]
pub fn resolve_use_colour(colour: bool, no_colour: bool, target: &OutputTarget) -> bool {
    if colour {
        return true;
    }
    if no_colour {
        return false;
    }
    target.is_stdout() && std::io::stdout().is_terminal()
}
