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

//! The smallest document that still reports one diagnostic.

use std::str::FromStr;

use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types::Uri;

use crate::Backend;

impl Backend {
    /// Does `source` still report `code`?
    ///
    /// The oracle the reducer tests each candidate against. It runs the plain
    /// analyser rather than the server's configured pipeline: a reproducer is
    /// for a bug report, so it has to stand on its own in a fresh checkout,
    /// not depend on the reporter's disabled-code set or spec packs.
    fn reproduces_diagnostic(source: &str, dialect: &str, code: &str) -> bool {
        tcl_compiler::analyser::Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .iter()
            .any(|d| d.code.to_string() == code)
    }

    /// Shrink `source` to the fewest lines that still report `code`.
    ///
    /// Delta debugging over lines, halving the granularity each pass: try
    /// removing each chunk, keep every removal the oracle still accepts, and
    /// when a pass at one granularity stops helping, halve the chunk and go
    /// again. That reaches a one-minimal result in far fewer analyser runs
    /// than removing one line at a time, which matters because each run is a
    /// full analysis.
    fn reduce_to_minimal(source: &str, dialect: &str, code: &str) -> Vec<String> {
        let mut lines: Vec<String> = source.lines().map(str::to_owned).collect();
        let mut chunk = lines.len().max(1);
        // A budget, so a pathological document cannot hold the blocking pool.
        let mut budget = 400_usize;

        while chunk >= 1 {
            let mut index = 0;
            while index < lines.len() {
                if budget == 0 {
                    return lines;
                }
                let end = (index + chunk).min(lines.len());
                let mut candidate = lines.clone();
                candidate.drain(index..end);
                budget -= 1;
                if !candidate.is_empty()
                    && Self::reproduces_diagnostic(&candidate.join("\n"), dialect, code)
                {
                    lines = candidate;
                } else {
                    index = end;
                }
            }
            if chunk == 1 {
                break;
            }
            chunk /= 2;
        }
        lines
    }

    /// Handle `tcl-lsp.minimizeDiagnostic`: the smallest document that still
    /// reports a diagnostic.
    ///
    /// Arguments are `[uri, code]`. Null when the document does not report
    /// that code at all, which is what the clients render as "could not build
    /// a minimal repro".
    ///
    /// `renamed` is always false. Shrinking is line-based; renaming
    /// identifiers would make a smaller reproducer but a less recognisable
    /// one, and the field exists so a future pass can say it did that.
    pub(in super::super) async fn minimize_diagnostic_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
            return Ok(None);
        };
        let Some(code) = args.get(1).and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };

        let text = doc.text.to_string();
        let dialect = doc.dialect.clone();
        let code = code.to_owned();
        let value = crate::rt::spawn_blocking(move || {
            // Nothing to minimise if the document does not report it. Saying
            // so beats returning the whole document as its own "reproducer".
            if !Self::reproduces_diagnostic(&text, &dialect, &code) {
                return None;
            }
            let original_lines = text.lines().count();
            let reduced = Self::reduce_to_minimal(&text, &dialect, &code);
            let source = format!("{}\n", reduced.join("\n"));
            Some(serde_json::json!({
                "code": code,
                "source": source,
                "originalLines": original_lines,
                "reducedLines": reduced.len(),
                "renamed": false,
                "reproduces": Self::reproduces_diagnostic(&source, &dialect, &code),
            }))
        })
        .await
        .unwrap_or(None);
        Ok(value)
    }
}
