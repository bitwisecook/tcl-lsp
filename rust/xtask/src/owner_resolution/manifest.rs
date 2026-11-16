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

//! Parsing the marked Markdown tables of the contract: the owner manifest, the internal manifest and the owner headings.

use super::{
    CONTRACT_PATH, END_MARKER, INTERNAL_END, INTERNAL_START, InternalOwnerRow, OwnerRow,
    START_MARKER,
};
use anyhow::Result;
use std::collections::BTreeSet;

pub(super) fn parse_manifest(markdown: &str) -> Result<Vec<OwnerRow>, String> {
    let start = markdown
        .find(START_MARKER)
        .ok_or_else(|| format!("missing `{START_MARKER}` in {CONTRACT_PATH}"))?;
    let body = &markdown[start + START_MARKER.len()..];
    let end = body
        .find(END_MARKER)
        .ok_or_else(|| format!("missing `{END_MARKER}` in {CONTRACT_PATH}"))?;

    let mut rows = Vec::new();
    for line in body[..end].lines() {
        let cells = split_row(line);
        if cells.is_empty() || cells[0] == "Surface" || cells.iter().all(|cell| *cell == "---") {
            continue;
        }
        if cells.len() != 5 {
            return Err(format!(
                "owner manifest row must have five columns: `{line}`"
            ));
        }
        let source_paths = code_tokens(cells[1]);
        let entry_points = code_tokens(cells[2]);
        let drift_gate = if cells[4] == "none" {
            None
        } else {
            let gates = code_tokens(cells[4]);
            if gates.len() != 1 {
                return Err(format!(
                    "owner manifest drift gate must be `none` or one backticked Make target: `{line}`"
                ));
            }
            gates.into_iter().next()
        };
        rows.push(OwnerRow {
            surface: cells[0].to_owned(),
            source_paths,
            entry_points,
            axis: cells[3].to_owned(),
            drift_gate,
        });
    }
    if rows.is_empty() {
        return Err(format!("owner manifest in {CONTRACT_PATH} has no rows"));
    }
    Ok(rows)
}

pub(super) fn parse_internal_manifest(markdown: &str) -> Result<Vec<InternalOwnerRow>, String> {
    let Some((_, tail)) = markdown.split_once(INTERNAL_START) else {
        return if markdown.contains(INTERNAL_END) {
            Err(format!("missing `{INTERNAL_START}`"))
        } else {
            Ok(Vec::new())
        };
    };
    if markdown.matches(INTERNAL_START).count() != 1 || markdown.matches(INTERNAL_END).count() != 1
    {
        return Err("internal owner manifest requires one matched marker pair".to_owned());
    }
    let (body, _) = tail
        .split_once(INTERNAL_END)
        .ok_or_else(|| format!("missing `{INTERNAL_END}`"))?;
    let mut rows = Vec::new();
    for line in body.lines() {
        let cells = split_row(line);
        if cells.is_empty() || cells[0] == "Surface" || cells.iter().all(|cell| *cell == "---") {
            continue;
        }
        if cells.len() != 10 || cells[0].is_empty() || cells[5] != "crate" {
            return Err(format!(
                "internal owner row requires ten columns and explicit supported scope `crate`: `{line}`"
            ));
        }
        rows.push(InternalOwnerRow {
            surface: cells[0].to_owned(),
            source: single_code_token(cells[1])?,
            crate_name: single_code_token(cells[2])?,
            module: single_code_token(cells[3])?,
            entry: single_code_token(cells[4])?,
            consumer_source: single_code_token(cells[6])?,
            consumer_module: single_code_token(cells[7])?,
            consumer_entry: single_code_token(cells[8])?,
            receiver: single_code_token(cells[9])?,
        });
    }
    if rows.is_empty() {
        return Err("internal owner manifest has no rows".to_owned());
    }
    Ok(rows)
}

pub(super) fn single_code_token(cell: &str) -> Result<String, String> {
    match code_tokens(cell).as_slice() {
        [value] => Ok(value.clone()),
        _ => Err(format!(
            "internal owner cell requires one backticked value: `{cell}`"
        )),
    }
}

pub(super) fn split_row(line: &str) -> Vec<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return Vec::new();
    }
    trimmed[1..trimmed.len() - 1]
        .split('|')
        .map(str::trim)
        .collect()
}

fn code_tokens(cell: &str) -> Vec<String> {
    cell.split('`')
        .enumerate()
        .filter_map(|(index, part)| (index % 2 == 1).then_some(part.trim().to_owned()))
        .filter(|part| !part.is_empty())
        .collect()
}

pub(super) fn owner_headings(markdown: &str) -> BTreeSet<String> {
    let Some(start) = markdown.find("## Owners") else {
        return BTreeSet::new();
    };
    let tail = &markdown[start..];
    let end = tail.find("\n## Decision rules").unwrap_or(tail.len());
    tail[..end]
        .lines()
        .filter_map(|line| line.strip_prefix("### `"))
        .filter_map(|line| line.split('`').next())
        .map(str::to_owned)
        .collect()
}

pub(super) fn declared_owner_bullets(markdown: &str) -> Vec<(String, String)> {
    let Some(start) = markdown.find("## Owners") else {
        return Vec::new();
    };
    let tail = &markdown[start..];
    let end = tail.find("\n## Decision rules").unwrap_or(tail.len());
    let mut owner = String::new();
    let mut out = Vec::new();
    for line in tail[..end].lines() {
        if let Some(heading) = line.strip_prefix("### `") {
            heading
                .split('`')
                .next()
                .unwrap_or_default()
                .clone_into(&mut owner);
        } else if let Some(bullet) = line.trim_start().strip_prefix("- `")
            && let Some(name) = bullet.split('`').next()
        {
            out.push((owner.clone(), name.to_owned()));
        }
    }
    out
}
