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

//! The source-of-truth check for the shared semantic-owner contract.
//!
//! The contract deliberately owns this manifest: a Rust table here would be
//! easy to update while leaving the human-facing map stale. The check parses
//! the small, marked Markdown table, then verifies every source path, entry
//! point, and named Makefile gate. It also makes sure the existing owner
//! headings are represented by a manifest row.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};

const CONTRACT_PATH: &str = "docs/design/contracts/shared-utility-contracts-rust.md";
const MAKEFILE_PATH: &str = "Makefile";
const XTASK_MAIN_PATH: &str = "rust/xtask/src/main.rs";
const START_MARKER: &str = "<!-- owner-resolution-manifest -->";
const END_MARKER: &str = "<!-- end-owner-resolution-manifest -->";

#[derive(Debug, PartialEq, Eq)]
struct OwnerRow {
    surface: String,
    source_paths: Vec<String>,
    entry_points: Vec<String>,
    axis: String,
    drift_gate: Option<String>,
}

/// Verify the owner manifest and its registered drift gates.
pub fn run() -> Result<ExitCode> {
    let root = crate::util::repo_root();
    let contract = read(&root, CONTRACT_PATH)?;
    let makefile = read(&root, MAKEFILE_PATH)?;
    let xtask_main = read(&root, XTASK_MAIN_PATH)?;

    let mut problems = validate_manifest(&root, &contract, &makefile, &xtask_main);
    if problems.is_empty() {
        let owner_count = parse_manifest(&contract)
            .map(|rows| rows.len())
            .unwrap_or_default();
        println!(
            "owner-resolution: OK ({owner_count} owner row(s) resolve to live source and gates)"
        );
        return Ok(ExitCode::SUCCESS);
    }

    problems.sort();
    eprintln!("owner-resolution: contract drift detected:");
    for problem in problems {
        eprintln!("  - {problem}");
    }
    Ok(ExitCode::FAILURE)
}

fn read(root: &Path, relative: &str) -> Result<String> {
    std::fs::read_to_string(root.join(relative))
        .with_context(|| format!("read owner-resolution input `{relative}`"))
}

fn validate_manifest(root: &Path, contract: &str, makefile: &str, xtask_main: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let rows = match parse_manifest(contract) {
        Ok(rows) => rows,
        Err(problem) => {
            problems.push(problem);
            return problems;
        }
    };

    let owner_headings = owner_headings(contract);
    let mut manifest_owners = BTreeSet::new();
    for row in &rows {
        for path in &row.source_paths {
            if let Some(owner) = owner_crate(path) {
                manifest_owners.insert(owner.to_owned());
            }
        }
    }
    for owner in owner_headings {
        if !manifest_owners.contains(&owner) {
            problems.push(format!(
                "owner heading `{owner}` has no row in the owner-resolution manifest"
            ));
        }
    }
    for (owner, declared) in declared_owner_bullets(contract) {
        let declared_name = declared.rsplit("::").next().unwrap_or(&declared);
        let covered = rows.iter().any(|row| {
            row.source_paths.iter().any(|path| {
                path.ends_with(&format!("/{declared_name}.rs"))
                    || path.contains(&format!("/{declared_name}/"))
            }) || row
                .entry_points
                .iter()
                .any(|entry| entry == &declared || entry == declared_name)
        });
        if !covered {
            problems.push(format!(
                "declared owner bullet `{owner}::{declared}` has no manifest row"
            ));
        }
    }

    for row in rows {
        if row.surface.is_empty() {
            problems.push("owner row has an empty surface name".to_owned());
        }
        if row.source_paths.is_empty() {
            problems.push(format!("owner `{}` has no source path", row.surface));
        }
        if row.entry_points.is_empty() {
            problems.push(format!("owner `{}` has no public entry point", row.surface));
        }
        if row.axis.is_empty() {
            problems.push(format!(
                "owner `{}` has no dialect/release axis",
                row.surface
            ));
        }

        let mut source_texts = Vec::new();
        for source in &row.source_paths {
            let path = root.join(source);
            match std::fs::read_to_string(&path) {
                Ok(text) => source_texts.push((source.as_str(), text)),
                Err(error) => problems.push(format!(
                    "owner `{}` source `{source}` is missing or unreadable: {error}",
                    row.surface
                )),
            }
        }
        for entry in &row.entry_points {
            let resolved_source = source_texts
                .iter()
                .find(|(_, source)| entry_is_declared(source, entry));
            if resolved_source.is_none() {
                problems.push(format!(
                    "owner `{}` entry point `{entry}` has no public declaration in its source paths",
                    row.surface,
                ));
            }
        }

        if let Some(gate) = row.drift_gate {
            let Some(dispatch_name) = make_gate_command(makefile, &gate) else {
                problems.push(format!(
                    "owner `{}` names missing or non-xtask Makefile drift gate `{gate}`",
                    row.surface
                ));
                continue;
            };
            if !xtask_dispatches(xtask_main, &dispatch_name) {
                problems.push(format!(
                    "owner `{}` gate `{gate}` invokes `{dispatch_name}` but xtask dispatch has no matching command",
                    row.surface,
                ));
            }
        }
    }
    problems
}

fn parse_manifest(markdown: &str) -> Result<Vec<OwnerRow>, String> {
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

fn split_row(line: &str) -> Vec<&str> {
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

fn owner_headings(markdown: &str) -> BTreeSet<String> {
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

fn declared_owner_bullets(markdown: &str) -> Vec<(String, String)> {
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

fn entry_is_declared(source: &str, entry: &str) -> bool {
    let ident = entry.rsplit("::").next().unwrap_or(entry);
    let public_declaration = source.lines().any(|line| {
        let trimmed = line.trim_start();
        [
            "pub fn ",
            "pub const ",
            "pub static ",
            "pub struct ",
            "pub enum ",
            "pub trait ",
            "pub type ",
            "pub use ",
        ]
        .iter()
        .any(|prefix| {
            trimmed.strip_prefix(prefix).is_some_and(|rest| {
                rest.starts_with(ident)
                    && rest
                        .as_bytes()
                        .get(ident.len())
                        .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
            })
        })
    });
    if public_declaration {
        return true;
    }
    let Some(owner) = entry
        .strip_suffix(ident)
        .and_then(|prefix| prefix.strip_suffix("::"))
    else {
        return false;
    };
    let mut trait_depth = None;
    let mut depth = 0usize;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trait_depth.is_none()
            && trimmed
                .strip_prefix("pub trait ")
                .is_some_and(|rest| starts_with_identifier(rest, owner))
        {
            trait_depth = Some(depth + line.bytes().filter(|&byte| byte == b'{').count());
        } else if let Some(start_depth) = trait_depth {
            if depth >= start_depth
                && trimmed
                    .strip_prefix("fn ")
                    .is_some_and(|rest| starts_with_identifier(rest, ident))
            {
                return true;
            }
            let opens = line.bytes().filter(|&byte| byte == b'{').count();
            let closes = line.bytes().filter(|&byte| byte == b'}').count();
            depth = depth.saturating_add(opens).saturating_sub(closes);
            if depth < start_depth {
                trait_depth = None;
            }
            continue;
        }
        let opens = line.bytes().filter(|&byte| byte == b'{').count();
        let closes = line.bytes().filter(|&byte| byte == b'}').count();
        depth = depth.saturating_add(opens).saturating_sub(closes);
    }
    false
}

fn xtask_dispatches(main: &str, command: &str) -> bool {
    let variant = match command {
        "resolution-drift" => "ResolutionDrift",
        "number-drift" => "NumberDrift",
        "audit-option-dialects" => "AuditOptionDialects",
        "command-backing" => "WasmBacking",
        "diag-tables" => "DiagTables",
        "gen-ai-diagnostics" => "GenAiDiagnostics",
        "gen-editor-extensions" => "GenEditorExtensions",
        "owner-resolution" => "OwnerResolution",
        "sslictcl-data" => "SslictclData",
        _ => return false,
    };
    let prefix = format!("Command::{variant}");
    main.lines().any(|line| {
        let trimmed = line.trim_start();
        if !trimmed.starts_with(&prefix) {
            return false;
        }
        // A single-line arm carries its `=>`; an arm whose struct pattern
        // binds fields opens a brace and puts the `=>` several lines later.
        trimmed.contains("=>") || trimmed.ends_with('{')
    })
}

fn starts_with_identifier(text: &str, identifier: &str) -> bool {
    text.strip_prefix(identifier).is_some_and(|rest| {
        rest.as_bytes()
            .first()
            .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
    })
}

fn owner_crate(path: &str) -> Option<&str> {
    let path = path.strip_prefix("rust/")?;
    path.split('/').next()
}

fn make_gate_command(makefile: &str, target: &str) -> Option<String> {
    direct_gate_command(makefile, target).or_else(|| {
        // A gate may be an alias: a `.PHONY` target whose whole body is one
        // prerequisite that carries the recipe (`xtask-sslictcl-data:
        // check-source-data`). The manifest should be free to name the gate a
        // reader would run, so follow the prerequisites one level rather than
        // forcing the contract to name the inner target.
        gate_prerequisites(makefile, target)
            .into_iter()
            .find_map(|prerequisite| direct_gate_command(makefile, &prerequisite))
    })
}

/// The `cargo xtask <verb>` this target's own recipe runs, if any.
fn direct_gate_command(makefile: &str, target: &str) -> Option<String> {
    let target_prefix = format!("{target}:");
    let mut in_recipe = false;
    for line in makefile.lines() {
        if !in_recipe {
            if line.trim_start().starts_with(&target_prefix) {
                in_recipe = true;
            }
            continue;
        }
        if !line.starts_with('\t') {
            if !line.trim().is_empty() {
                break;
            }
            continue;
        }
        let Some(command) = line.split_once("cargo xtask ").map(|(_, rest)| rest) else {
            continue;
        };
        return command.split_whitespace().next().map(str::to_owned);
    }
    None
}

/// The prerequisites named on `target`'s rule line, ignoring a trailing
/// `## help` comment. Empty when the target has no rule (or only `.PHONY`
/// declarations mention it, which never carry a `:` in that position).
fn gate_prerequisites(makefile: &str, target: &str) -> Vec<String> {
    let target_prefix = format!("{target}:");
    for line in makefile.lines() {
        if line.starts_with(&target_prefix) {
            let rest = &line[target_prefix.len()..];
            let rest = rest.split_once("##").map_or(rest, |(head, _)| head);
            return rest.split_whitespace().map(str::to_owned).collect();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_marked_manifest_rows() {
        let md = "<!-- owner-resolution-manifest -->\n| Surface | Owner source paths | Public entry points | Axis | Drift gate |\n| --- | --- | --- | --- | --- |\n| names | `rust/a/src/lib.rs`; `rust/b/src/lib.rs` | `one`; `two` | invariant | `xtask-example` |\n<!-- end-owner-resolution-manifest -->";
        let rows = parse_manifest(md).expect("manifest parses");
        assert_eq!(
            rows,
            vec![OwnerRow {
                surface: "names".to_owned(),
                source_paths: vec![
                    "rust/a/src/lib.rs".to_owned(),
                    "rust/b/src/lib.rs".to_owned()
                ],
                entry_points: vec!["one".to_owned(), "two".to_owned()],
                axis: "invariant".to_owned(),
                drift_gate: Some("xtask-example".to_owned()),
            }]
        );
    }

    #[test]
    fn a_multi_line_dispatch_arm_still_counts() {
        let main = "        Command::SslictclData {\n            operation,\n        } => sslictcl_data::run(&operation),\n";
        assert!(xtask_dispatches(main, "sslictcl-data"));
        assert!(!xtask_dispatches(main, "owner-resolution"));
    }

    #[test]
    fn a_gate_alias_resolves_through_its_prerequisite() {
        let makefile = "xtask-alias: real-gate ## help text\n\nreal-gate:\n\tcd $(ROOT) && cargo xtask sslictcl-data check\n";
        assert_eq!(
            make_gate_command(makefile, "xtask-alias").as_deref(),
            Some("sslictcl-data"),
        );
        assert_eq!(
            make_gate_command(makefile, "real-gate").as_deref(),
            Some("sslictcl-data"),
        );
        assert_eq!(make_gate_command(makefile, "no-such-target"), None);
    }

    #[test]
    fn owner_headings_are_limited_to_owner_section() {
        let md = "## Owners\n### `tcl-syntax` — owner\n## Decision rules\n### `not-an-owner`";
        assert_eq!(
            owner_headings(md),
            BTreeSet::from(["tcl-syntax".to_owned()])
        );
    }

    #[test]
    fn missing_declared_owner_row_is_reported() {
        let root = crate::util::repo_root();
        let contract = std::fs::read_to_string(root.join(CONTRACT_PATH)).expect("contract");
        let contract = contract
            .lines()
            .filter(|line| !line.starts_with("| glob matching |"))
            .collect::<Vec<_>>()
            .join("\n");
        let makefile = std::fs::read_to_string(root.join(MAKEFILE_PATH)).expect("Makefile");
        let main = std::fs::read_to_string(root.join(XTASK_MAIN_PATH)).expect("xtask main");
        let problems = validate_manifest(&root, &contract, &makefile, &main);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("tcl-syntax::glob"))
        );
    }

    #[test]
    fn missing_source_is_reported() {
        let root = crate::util::repo_root();
        let contract = synthetic_manifest("`rust/tcl-syntax/src/moved.rs`", "`Moved`", "none");
        let makefile = std::fs::read_to_string(root.join(MAKEFILE_PATH)).expect("Makefile");
        let main = std::fs::read_to_string(root.join(XTASK_MAIN_PATH)).expect("xtask main");
        let problems = validate_manifest(&root, &contract, &makefile, &main);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("missing or unreadable"))
        );
    }

    #[test]
    fn missing_entry_point_is_reported() {
        let root = crate::util::repo_root();
        let contract = synthetic_manifest(
            "`rust/tcl-syntax/src/list.rs`",
            "`DefinitelyMissing`",
            "none",
        );
        let makefile = std::fs::read_to_string(root.join(MAKEFILE_PATH)).expect("Makefile");
        let main = std::fs::read_to_string(root.join(XTASK_MAIN_PATH)).expect("xtask main");
        let problems = validate_manifest(&root, &contract, &makefile, &main);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("no public declaration"))
        );
    }

    #[test]
    fn private_functions_do_not_satisfy_public_entry_points() {
        assert!(!entry_is_declared("fn hidden() {}", "hidden"));
        assert!(!entry_is_declared(
            "pub trait ValueOpsExtra {\n    fn dict_pairs_extra(&self);\n}",
            "ValueOps::dict_pairs"
        ));
        assert!(!entry_is_declared(
            "pub trait ValueOps {\n    fn dict_pairs_extra(&self);\n}",
            "ValueOps::dict_pairs"
        ));
        assert!(entry_is_declared(
            "pub trait Public {\n    fn member(&self);\n}",
            "Public::member"
        ));
    }

    #[test]
    fn missing_make_gate_is_reported() {
        let root = crate::util::repo_root();
        let contract = synthetic_manifest(
            "`rust/tcl-syntax/src/list.rs`",
            "`split_list`",
            "`xtask-does-not-exist`",
        );
        let makefile = std::fs::read_to_string(root.join(MAKEFILE_PATH)).expect("Makefile");
        let main = std::fs::read_to_string(root.join(XTASK_MAIN_PATH)).expect("xtask main");
        let problems = validate_manifest(&root, &contract, &makefile, &main);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("missing or non-xtask Makefile drift gate"))
        );
    }

    fn synthetic_manifest(paths: &str, entries: &str, gate: &str) -> String {
        format!(
            "## Owners\n### `tcl-syntax` — owner\n\n<!-- owner-resolution-manifest -->\n| Surface | Owner source paths | Public entry points | Dialect/release axis | Drift gate |\n| --- | --- | --- | --- | --- |\n| test | {paths} | {entries} | invariant | {gate} |\n<!-- end-owner-resolution-manifest -->\n\n## Decision rules"
        )
    }

    #[test]
    fn resolves_make_targets_to_their_actual_xtask_commands() {
        let makefile = "xtask-resolution-drift:\n\tcd $(ROOT) && cargo xtask resolution-drift\n\nxtask-option-registry-drift:\n\tcd $(ROOT) && cargo xtask audit-option-dialects --check\n";
        assert_eq!(
            make_gate_command(makefile, "xtask-resolution-drift"),
            Some("resolution-drift".to_owned())
        );
        assert_eq!(
            make_gate_command(makefile, "xtask-option-registry-drift"),
            Some("audit-option-dialects".to_owned())
        );
    }

    #[test]
    fn dispatch_check_ignores_comments_and_requires_match_arm() {
        assert!(!xtask_dispatches(
            "// Command::ResolutionDrift => resolution_drift::run(check),",
            "resolution-drift"
        ));
        assert!(xtask_dispatches(
            "Command::ResolutionDrift { check } => resolution_drift::run(check),",
            "resolution-drift"
        ));
    }
}
