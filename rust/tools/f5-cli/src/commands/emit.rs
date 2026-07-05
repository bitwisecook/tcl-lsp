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

//! Shared output-format plumbing for config-producing verbs.
//!
//! `render_config` renders an SCF text as either SCF (verbatim), a full
//! `tmsh create|modify` script, or a `tmsh-delta`. When the input fails to
//! parse into any object it warns on stderr and falls back to the raw text.

use tcl_bigip::parser::{BigipConfig, parse_bigip_conf};
use tcl_bigip::tmsh_emit::{DEFAULT_ORDER, emit_tmsh, emit_tmsh_delta};

/// Render `text` as `fmt` (`"scf"` / `"tmsh"` / `"tmsh-delta"`).
///
/// `tmsh_verb` is `"create"` (extractive verbs) or `"modify"` (in-place
/// rewriters). `original` is the pre-edit text for `tmsh-delta`.
/// `transaction` wraps tmsh output in a `cli transaction` envelope.
#[must_use]
pub fn render_config(
    text: &str,
    fmt: &str,
    tmsh_verb: &str,
    transaction: bool,
    original: &str,
) -> String {
    if fmt == "scf" {
        return text.to_owned();
    }
    let cfg = parse_bigip_conf(text, "Common");
    if !has_any_object(&cfg) {
        eprintln!(
            "warning: --format {fmt}: input did not parse as a BIG-IP config; \
             emitting raw text instead."
        );
        return text.to_owned();
    }
    let script = if fmt == "tmsh-delta" {
        let old_cfg = if original.is_empty() {
            BigipConfig::default()
        } else {
            parse_bigip_conf(original, "Common")
        };
        emit_tmsh_delta(&old_cfg, &cfg, original, text, &DEFAULT_ORDER)
    } else {
        emit_tmsh(&cfg, text, &DEFAULT_ORDER, tmsh_verb == "modify")
    };
    if transaction {
        wrap_tmsh_transaction(&script.text)
    } else {
        script.text
    }
}

/// Wrap `tmsh_script` in F5's `cli transaction` envelope. Idempotent;
/// preserves a leading comment / blank banner above the envelope.
#[must_use]
pub fn wrap_tmsh_transaction(tmsh_script: &str) -> String {
    if tmsh_script.trim_start().starts_with("cli transaction") {
        return tmsh_script.to_owned();
    }
    let lines: Vec<&str> = tmsh_script.split('\n').collect();
    // `splitlines(keepends=False)` drops a single trailing newline; emulate by
    // ignoring a final empty element produced by the trailing `\n`.
    let lines: &[&str] = if lines.last() == Some(&"") {
        &lines[..lines.len() - 1]
    } else {
        &lines
    };
    let mut head_lines: Vec<&str> = Vec::new();
    let mut body_start = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            head_lines.push(line);
            body_start = i + 1;
        } else {
            break;
        }
    }
    let body_lines = &lines[body_start..];
    if body_lines.is_empty() {
        return tmsh_script.to_owned();
    }
    let head = head_lines.join("\n");
    let body = body_lines.join("\n");
    let head_block = if head.is_empty() {
        String::new()
    } else {
        format!("{head}\n")
    };
    format!("{head_block}cli transaction\n{body}\nsubmit-transaction\n")
}

/// True when `cfg` parsed into at least one structured object. The
/// typed-object inventory is sufficient — every relevant kind lands in
/// `cfg.objects`, and `generic_objects` covers the long tail.
fn has_any_object(cfg: &BigipConfig) -> bool {
    !cfg.objects.is_empty() || !cfg.generic_objects.is_empty()
}
