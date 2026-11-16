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

//! The smoke manifest: its rows, the target that owns each smoke source and the validation of both.

use super::{ManifestRow, SUPPORTED_KINDS, SmokeInventory, Target, TargetIdentity};
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

fn ownership_rank(source: &Path, target: &Target) -> Option<u8> {
    if source == target.source {
        return Some(3);
    }
    if target.source.extension() == Some(OsStr::new("rs")) {
        let sibling_modules = target.source.with_extension("");
        if source.starts_with(sibling_modules) {
            return Some(2);
        }
    }
    if matches!(target.kind.as_str(), "test" | "example" | "bench") {
        return None;
    }
    let root_name = target.source.file_name()?;
    let root_parent = target.source.parent()?;
    if root_name == OsStr::new("main.rs")
        && root_parent.parent()?.file_name() == Some(OsStr::new("bin"))
    {
        return source.starts_with(root_parent).then_some(2);
    }
    if matches!(root_name.to_str(), Some("lib.rs" | "main.rs")) && source.starts_with(root_parent) {
        return Some(1);
    }
    None
}

pub(super) fn best_owners<'a>(source: &Path, targets: &'a [Target]) -> Vec<&'a Target> {
    let ranked: Vec<(u8, &Target)> = targets
        .iter()
        .filter_map(|target| ownership_rank(source, target).map(|rank| (rank, target)))
        .collect();
    let Some(best) = ranked.iter().map(|(rank, _)| *rank).max() else {
        return Vec::new();
    };
    ranked
        .into_iter()
        .filter_map(|(rank, target)| (rank == best).then_some(target))
        .collect()
}

pub(super) fn target_identity(target: &Target) -> TargetIdentity {
    (
        target.package.clone(),
        target.kind.clone(),
        target.name.clone(),
    )
}

pub(super) fn manifest_owners<'a>(
    source: &Path,
    relative: &str,
    targets: &'a [Target],
    inventory: &SmokeInventory,
) -> Vec<&'a Target> {
    let Some(identities) = inventory.owners.get(relative) else {
        return best_owners(source, targets);
    };
    targets
        .iter()
        .filter(|target| identities.contains(&target_identity(target)))
        .collect()
}

pub(super) fn is_smoke_named_target(target: &Target) -> bool {
    target.name == "smoke" || target.name.ends_with("_smoke")
}

pub(super) fn read_manifest(manifest: &Path) -> Result<Vec<ManifestRow>> {
    let text =
        fs::read_to_string(manifest).with_context(|| format!("reading {}", manifest.display()))?;
    let mut rows = Vec::new();
    let mut seen = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 4 {
            bail!(
                "{}:{}: expected four tab-separated fields",
                manifest.display(),
                index + 1
            );
        }
        let line_number = index + 1;
        if let Some(first_line) =
            seen.insert((fields[0], fields[1], fields[2], fields[3]), line_number)
        {
            bail!(
                "{}:{line_number}: duplicate smoke target row (first declared at line {first_line})",
                manifest.display()
            );
        }
        rows.push(ManifestRow {
            source_text: fields[0].to_owned(),
            package: fields[1].to_owned(),
            kind: fields[2].to_owned(),
            target_name: fields[3].to_owned(),
        });
    }
    Ok(rows)
}

pub(super) fn validate_manifest(
    root: &Path,
    manifest: &Path,
    package_roots: &HashMap<String, PathBuf>,
    targets: &[Target],
    inventory: &SmokeInventory,
) -> Result<Vec<ManifestRow>> {
    let rows = read_manifest(manifest)?;
    let mut errors = Vec::new();
    let mut declared = BTreeSet::new();
    for row in &rows {
        declared.insert((
            row.package.clone(),
            row.kind.clone(),
            row.target_name.clone(),
        ));
        let source = root.join(&row.source_text);
        if !source.is_file() {
            errors.push(format!("missing smoke source: {}", row.source_text));
            continue;
        }
        if !package_roots.contains_key(&row.package) {
            errors.push(format!(
                "unknown Cargo package '{}' for {}",
                row.package, row.source_text
            ));
            continue;
        }
        if !SUPPORTED_KINDS.contains(&row.kind.as_str()) {
            errors.push(format!(
                "invalid smoke target kind '{}' for {}",
                row.kind, row.source_text
            ));
            continue;
        }
        let package_targets: Vec<Target> = targets
            .iter()
            .filter(|target| target.package == row.package)
            .cloned()
            .collect();
        let owners = manifest_owners(&source, &row.source_text, &package_targets, inventory);
        let exact: Vec<&Target> = owners
            .iter()
            .filter(|owner| owner.kind == row.kind && owner.name == row.target_name)
            .copied()
            .collect();
        if exact.len() == 1 {
            if !exact[0].testable {
                errors.push(format!(
                    "smoke source {} belongs to Cargo target {}:{} with test = false",
                    row.source_text, row.kind, row.target_name
                ));
            }
            continue;
        }
        if owners.is_empty() {
            errors.push(format!(
                "no Cargo target owns smoke source {}",
                row.source_text
            ));
        } else if owners.len() > 1 {
            let names = owners
                .iter()
                .map(|owner| format!("{}:{}", owner.kind, owner.name))
                .collect::<Vec<_>>()
                .join(", ");
            errors.push(format!(
                "ambiguous Cargo target ownership for {}: {names}; move the smoke test to a target root or integration test",
                row.source_text
            ));
        } else {
            errors.push(format!(
                "smoke source {} belongs to {}:{}, not {}:{}",
                row.source_text, owners[0].kind, owners[0].name, row.kind, row.target_name
            ));
        }
    }

    for target in targets
        .iter()
        .filter(|target| target.testable && is_smoke_named_target(target))
    {
        let identity = (
            target.package.clone(),
            target.kind.clone(),
            target.name.clone(),
        );
        if !declared.contains(&identity) {
            errors.push(format!(
                "smoke-named Cargo target {} {}:{} has no smoke-targets.tsv row",
                target.package, target.kind, target.name
            ));
        }
    }
    if errors.is_empty() {
        Ok(rows)
    } else {
        bail!("{}", errors.join("\n"))
    }
}
