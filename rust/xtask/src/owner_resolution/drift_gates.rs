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

//! The drift gate a row names: the Makefile target, the `cargo xtask` verb it runs and the naming proof gate.

use std::path::Path;

pub(super) fn xtask_dispatches(main: &str, command: &str) -> bool {
    let variant = match command {
        "resolution-drift" => "ResolutionDrift",
        "segmentation-drift" => "SegmentationDrift",
        "number-drift" => "NumberDrift",
        "audit-option-dialects" => "AuditOptionDialects",
        "command-backing" => "WasmBacking",
        "diag-tables" => "DiagTables",
        "gen-ai-diagnostics" => "GenAiDiagnostics",
        "gen-editor-catalogs" => "GenEditorCatalogs",
        "gen-editor-extensions" => "GenEditorExtensions",
        "owner-resolution" => "OwnerResolution",
        "sslictcl-data" => "SslictclData",
        "value-transfers" => "ValueTransfers",
        "registry-axes" => "RegistryAxes",
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

pub(super) fn naming_proof_gate_is_current(root: &Path, makefile: &str) -> bool {
    let lines: Vec<_> = makefile.lines().collect();
    let mut rules = lines.iter().enumerate().filter_map(|(index, line)| {
        if line.trim_start().starts_with('#') {
            return None;
        }
        let (targets, prerequisites) = line.split_once(':')?;
        targets
            .split_whitespace()
            .any(|target| target == "check-name-resolution-proofs")
            .then_some((index, targets.trim(), prerequisites))
    });
    let Some((index, target, prerequisites)) = rules.next() else {
        return false;
    };
    let prerequisites = prerequisites.split('#').next().unwrap_or_default();
    if rules.next().is_some()
        || target != "check-name-resolution-proofs"
        || prerequisites.trim() != "check-tcl-evidence"
    {
        return false;
    }
    // The legacy name remains an alias; evidence is checked without digests.
    let alias_has_no_recipe = lines[index + 1..]
        .iter()
        .take_while(|line| {
            line.starts_with('\t') || line.starts_with('#') || line.trim().is_empty()
        })
        .all(|line| line.trim().is_empty() || line.trim_start().starts_with('#'));
    let mut evidence_rules = lines.iter().enumerate().filter_map(|(index, line)| {
        let (target, prerequisites) = line.split_once(':')?;
        (target.trim() == "check-tcl-evidence"
            && prerequisites
                .split('#')
                .next()
                .unwrap_or_default()
                .trim()
                .is_empty())
        .then_some(index)
    });
    let Some(evidence_index) = evidence_rules.next() else {
        return false;
    };
    alias_has_no_recipe
        && evidence_rules.next().is_none()
        && lines[evidence_index + 1..]
            .iter()
            .take_while(|line| line.starts_with('\t') || line.trim().is_empty())
            .filter(|line| !line.trim().is_empty())
            .copied()
            .eq([
                "\tcd $(ROOT) && python3 scripts/dev/check-tcl-evidence.py",
                "\tcd $(ROOT) && python3 -m unittest discover -s scripts/dev -p 'test_tcl_evidence*.py'",
            ])
        && root.join("scripts/dev/check-tcl-evidence.py").is_file()
}

pub(super) fn make_gate_command(makefile: &str, target: &str) -> Option<String> {
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
