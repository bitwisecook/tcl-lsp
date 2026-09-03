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

//! Project-source dependency discovery for `tcl pkg discover`.
//!
//! The full analyser owns the occurrence inventory: unlike the lightweight
//! signature scan it reaches ordinary procedure, method, namespace, and
//! nested-substitution bodies.  A standard optimiser pass then refines the
//! same source without dead-code elimination, allowing SCCP, constant
//! propagation, interpolation folding, and registry-declared pure builtin
//! folds to turn otherwise-dynamic package words into constants.  Findings
//! retain their original analyser spans; if the two inventories ever cease to
//! align, discovery conservatively falls back to the original one.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;
use tcl_compiler::analyser::Analyser;
use tcl_compiler::optimiser::optimise_source_multipass_filtered;
use tcl_compiler::optimiser::profiles::{OptimisationProfile, profile_to_disabled};
use tcl_pkg::manifest::{ManifestAst, load_manifest, load_manifest_text};
use tcl_pkg::version::Version;

use crate::cli::PkgCommon;

/// Directories which contain generated, installed, or vendored code rather
/// than the root project's own direct dependency declarations.
const SKIP_DIRECTORIES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".venv",
    ".tclpkg-build-home",
    "__pycache__",
    "build",
    "dist",
    "lib",
    "node_modules",
    "target",
    "tmp",
    "vendor",
];

#[derive(Debug, Clone, Serialize)]
struct RequirementReport {
    name: Option<String>,
    minimum: Option<String>,
    expression: String,
    version_expression: Option<String>,
    file: String,
    line: usize,
    conditional: bool,
    exact: bool,
    resolution: &'static str,
    status: &'static str,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AddedRequirement {
    name: String,
    minimum: String,
}

#[derive(Debug, Serialize)]
struct DiscoveryOutput {
    manifest: String,
    scanned_files: usize,
    requirements: Vec<RequirementReport>,
    added: Vec<AddedRequirement>,
    warnings: Vec<String>,
}

struct ResolvedRequirement {
    name: Option<String>,
    minimum: Option<String>,
    expression: String,
    version_expression: Option<String>,
    conditional: bool,
    exact: bool,
    resolution: &'static str,
    line: usize,
}

/// Run `tcl pkg discover`.
#[allow(clippy::too_many_arguments)]
pub fn run(
    inputs: &[PathBuf],
    add: bool,
    recursive: bool,
    dialect: Option<&str>,
    common: &PkgCommon,
    manifest_path: &Path,
) -> anyhow::Result<u8> {
    let manifest = match load_manifest(manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("error: {error}");
            return Ok(1);
        }
    };
    let project_dir = manifest_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let project_dir = std::fs::canonicalize(&project_dir).unwrap_or(project_dir);
    let roots = if inputs.is_empty() {
        vec![project_dir.clone()]
    } else {
        inputs.to_vec()
    };
    let source_paths = match collect_source_paths(&roots, recursive, manifest_path) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("error: {error:#}");
            return Ok(1);
        }
    };

    let explicit_dialect = tcl_cli_support::resolve_dialect(dialect)?;
    let documents = if source_paths.is_empty() {
        Vec::new()
    } else {
        tcl_cli_support::read_input_documents(&source_paths, &[], false)?
    };

    let declared: BTreeSet<String> = manifest
        .requires
        .iter()
        .chain(&manifest.dev_requires)
        .map(|requirement| requirement.name.clone())
        .collect();
    let mut reports = Vec::new();
    let mut warnings = Vec::new();
    let mut scanned_files = 0;

    for document in &documents {
        if document.abstains_on_encoding() {
            warnings.push(format!(
                "{} is not UTF-8 text; package discovery skipped it",
                display_path(document.path.as_deref(), &project_dir, &document.label)
            ));
            continue;
        }
        scanned_files += 1;
        let profile = document.effective_dialect(explicit_dialect);
        let registry = tcl_cli_support::registry_for_dialect(profile.name);
        let file = display_path(document.path.as_deref(), &project_dir, &document.label);
        let (requirements, warning) = discover_document_requirements(
            &document.source,
            &file,
            profile,
            &registry,
            tcl_cli_support::spec_pack_key(profile.name),
        );
        if let Some(warning) = warning {
            warnings.push(warning);
        }
        for requirement in requirements {
            reports.push(classify(requirement, &file, &manifest, &declared));
        }
    }

    reports.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.line.cmp(&right.line))
            .then_with(|| left.expression.cmp(&right.expression))
    });

    let additions = collect_additions(&reports);
    let added = if add && !additions.is_empty() {
        match append_requirements(manifest_path, &additions) {
            Ok(()) => additions
                .into_iter()
                .map(|(name, (_, minimum))| AddedRequirement { name, minimum })
                .collect(),
            Err(error) => {
                eprintln!("error: {error:#}");
                return Ok(1);
            }
        }
    } else {
        Vec::new()
    };

    let output = DiscoveryOutput {
        manifest: manifest_path.display().to_string(),
        scanned_files,
        requirements: reports,
        added,
        warnings,
    };
    if common.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_text(&output, add);
    }
    Ok(0)
}

fn discover_document_requirements(
    source: &str,
    file: &str,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &tcl_registry::CommandRegistry,
    pack_overlay: u64,
) -> (Vec<ResolvedRequirement>, Option<String>) {
    let original = Analyser::new()
        .with_file_path(Some(file.to_owned()))
        .with_pack_overlay(pack_overlay)
        .analyse(source, profile.name)
        .package_requires;

    // `standard` enables the optimiser's constant-analysis family while
    // leaving DCE and code motion off. One pass preserves the analyser's
    // occurrence order and count, which lets the refined values retain the
    // original source spans below.
    let disabled: HashSet<String> = profile_to_disabled(OptimisationProfile::Standard)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let (optimised, _, _) = optimise_source_multipass_filtered(
        source,
        registry,
        Some(profile),
        OptimisationProfile::Standard.max_iterations(),
        &disabled,
    );
    let refined = Analyser::new()
        .with_file_path(Some(file.to_owned()))
        .with_pack_overlay(pack_overlay)
        .analyse(&optimised, profile.name)
        .package_requires;

    let aligned = original.len() == refined.len();
    let warning = (!aligned).then(|| {
        format!(
            "{file}: optimiser changed the package-require inventory; dynamic refinement was skipped"
        )
    });
    let refined = aligned.then_some(refined);

    let requirements = original
        .into_iter()
        .enumerate()
        .map(|(index, raw)| {
            let candidate = refined
                .as_ref()
                .and_then(|items| items.get(index))
                .unwrap_or(&raw);
            let raw_name = static_word(&raw.name);
            let refined_name = static_word(&candidate.name);
            let raw_version = raw.version.as_deref().and_then(static_word);
            let refined_version = candidate.version.as_deref().and_then(static_word);
            let name = raw_name.clone().or(refined_name);
            let version = match raw.version.as_deref() {
                None => Some("0.0.1".to_owned()),
                Some(_) => raw_version.clone().or(refined_version),
            };
            let unresolved = name.is_none() || (raw.version.is_some() && version.is_none());
            let optimised_resolution = !unresolved
                && (raw_name.is_none()
                    || (raw.version.is_some() && raw_version.is_none())
                    || name.as_deref() != raw_name.as_deref()
                    || version.as_deref() != raw_version.as_deref());
            ResolvedRequirement {
                name,
                minimum: version,
                expression: raw.name,
                version_expression: raw.version,
                conditional: raw.conditional,
                exact: raw.exact,
                resolution: if unresolved {
                    "unresolved"
                } else if optimised_resolution {
                    "optimiser"
                } else {
                    "literal"
                },
                line: line_number(source, raw.range.start()),
            }
        })
        .collect();
    (requirements, warning)
}

fn classify(
    requirement: ResolvedRequirement,
    file: &str,
    manifest: &ManifestAst,
    declared: &BTreeSet<String>,
) -> RequirementReport {
    let mut report = RequirementReport {
        name: requirement.name,
        minimum: requirement.minimum,
        expression: requirement.expression,
        version_expression: requirement.version_expression,
        file: file.to_owned(),
        line: requirement.line,
        conditional: requirement.conditional,
        exact: requirement.exact,
        resolution: requirement.resolution,
        status: "candidate",
        reason: None,
    };

    let Some(name) = report.name.as_deref() else {
        report.status = "unresolved";
        report.reason = Some("package name is not statically resolvable".to_owned());
        return report;
    };
    if !manifest_atom(name) {
        report.status = "unresolved";
        report.reason = Some("resolved name cannot be represented safely in tclpkg.tcl".to_owned());
        return report;
    }
    if name == "Tcl" {
        report.status = "runtime";
        report.reason = Some("core Tcl is represented by the manifest's tcl directive".to_owned());
        return report;
    }
    if name == manifest.name {
        report.status = "self";
        report.reason = Some("the project cannot depend on itself".to_owned());
        return report;
    }
    if declared.contains(name) {
        report.status = "declared";
        report.reason = Some("already present in require or dev-require".to_owned());
        return report;
    }
    if report.conditional {
        report.status = "review";
        report.reason = Some(
            "guarded requirement may be an optional dependency; add it explicitly if needed"
                .to_owned(),
        );
        return report;
    }
    if report.exact {
        report.status = "review";
        report.reason = Some("tclpkg.tcl requirements cannot express -exact".to_owned());
        return report;
    }
    let Some(requirement_text) = report.minimum.as_deref() else {
        report.status = "unresolved";
        report.reason = Some("version requirement is not statically resolvable".to_owned());
        return report;
    };
    if tcl_registry::version::requirement_upper_bound(requirement_text).is_some() {
        report.status = "review";
        report.reason = Some("tclpkg.tcl requirements cannot express an upper bound".to_owned());
        return report;
    }
    let minimum = tcl_registry::version::requirement_lower_bound(requirement_text);
    if Version::parse(minimum).is_err() {
        report.status = "unresolved";
        report.reason = Some(format!(
            "invalid or unsupported version requirement: {requirement_text}"
        ));
        return report;
    }
    report.minimum = Some(minimum.to_owned());
    report
}

fn collect_additions(reports: &[RequirementReport]) -> BTreeMap<String, (Version, String)> {
    let mut additions: BTreeMap<String, (Version, String)> = BTreeMap::new();
    for report in reports.iter().filter(|report| report.status == "candidate") {
        let (Some(name), Some(minimum)) = (&report.name, &report.minimum) else {
            continue;
        };
        let Ok(parsed) = Version::parse(minimum) else {
            continue;
        };
        match additions.get(name) {
            Some((current, _)) if current >= &parsed => {}
            _ => {
                additions.insert(name.clone(), (parsed, minimum.clone()));
            }
        }
    }
    additions
}

fn append_requirements(
    path: &Path,
    additions: &BTreeMap<String, (Version, String)>,
) -> anyhow::Result<()> {
    let original = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read manifest {}", path.display()))?;
    let mut proposed = original.trim_end_matches(['\r', '\n']).to_owned();
    for (name, (_, minimum)) in additions {
        proposed.push('\n');
        write!(&mut proposed, "require {name} {minimum}").expect("writing to a String cannot fail");
    }
    proposed.push('\n');
    load_manifest_text(&proposed, Some(&path.to_string_lossy()))
        .context("discovered requirements would produce an invalid manifest")?;
    std::fs::write(path, proposed)
        .with_context(|| format!("cannot update manifest {}", path.display()))
}

fn print_text(output: &DiscoveryOutput, add: bool) {
    println!(
        "Scanned {} Tcl source file(s); found {} package requirement(s).",
        output.scanned_files,
        output.requirements.len()
    );
    for requirement in &output.requirements {
        let name = requirement
            .name
            .as_deref()
            .unwrap_or(requirement.expression.as_str());
        let minimum = requirement.minimum.as_deref().unwrap_or("?");
        println!(
            "  {name} {minimum}  [{}; {}]  {}:{}",
            requirement.status, requirement.resolution, requirement.file, requirement.line
        );
        if let Some(reason) = &requirement.reason {
            println!("    {reason}");
        }
    }
    for warning in &output.warnings {
        eprintln!("warning: {warning}");
    }
    if !output.added.is_empty() {
        println!("Added to {}:", output.manifest);
        for requirement in &output.added {
            println!("  {} {}", requirement.name, requirement.minimum);
        }
        println!("Run 'tcl pkg install' to resolve and update tclpkg.lock.");
    } else if add {
        println!("No safe undeclared requirements to add.");
    } else if output
        .requirements
        .iter()
        .any(|requirement| requirement.status == "candidate")
    {
        println!("Run 'tcl pkg discover --add' to add safe findings.");
    }
    if output
        .requirements
        .iter()
        .any(|requirement| matches!(requirement.status, "review" | "unresolved"))
    {
        println!("Use 'tcl pkg add NAME VERSION' for review-needed dependencies.");
    }
}

fn static_word(word: &str) -> Option<String> {
    (!tcl_compiler::naming::is_dynamic_word(word)).then(|| word.to_owned())
}

fn manifest_atom(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().any(|character| {
            character.is_whitespace()
                || matches!(
                    character,
                    ';' | '$' | '[' | ']' | '{' | '}' | '"' | '\\' | '#'
                )
        })
}

fn line_number(source: &str, offset: u32) -> usize {
    source
        .get(
            ..usize::try_from(offset)
                .unwrap_or(source.len())
                .min(source.len()),
        )
        .map_or(1, |prefix| {
            prefix.bytes().filter(|byte| *byte == b'\n').count() + 1
        })
}

fn display_path(path: Option<&Path>, project_dir: &Path, fallback: &str) -> String {
    path.and_then(|path| path.strip_prefix(project_dir).ok())
        .filter(|path| !path.as_os_str().is_empty())
        .map_or_else(|| fallback.to_owned(), |path| path.display().to_string())
}

fn collect_source_paths(
    roots: &[PathBuf],
    recursive: bool,
    manifest_path: &Path,
) -> anyhow::Result<Vec<PathBuf>> {
    let manifest = std::fs::canonicalize(manifest_path).unwrap_or_else(|_| manifest_path.into());
    let mut paths = BTreeSet::new();
    for root in roots {
        collect_source_path(root, recursive, true, &manifest, &mut paths)?;
    }
    Ok(paths.into_iter().collect())
}

fn collect_source_path(
    path: &Path,
    recursive: bool,
    explicit: bool,
    manifest_path: &Path,
    paths: &mut BTreeSet<PathBuf>,
) -> anyhow::Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("cannot inspect source input {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_file() {
        if !is_tcl_source(path) {
            if explicit {
                anyhow::bail!("unsupported source file: {}", path.display());
            }
            return Ok(());
        }
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if canonical != manifest_path {
            paths.insert(canonical);
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        anyhow::bail!("unsupported source input: {}", path.display());
    }

    let mut entries = std::fs::read_dir(path)
        .with_context(|| format!("cannot read source directory {}", path.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let child = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if !recursive || should_skip_directory(&child) {
                continue;
            }
            collect_source_path(&child, true, false, manifest_path, paths)?;
        } else if file_type.is_file() {
            collect_source_path(&child, false, false, manifest_path, paths)?;
        }
    }
    Ok(())
}

fn should_skip_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.') || SKIP_DIRECTORIES.contains(&name))
}

fn is_tcl_source(path: &Path) -> bool {
    if path.file_name().is_some_and(|name| name == "pkgIndex.tcl") {
        return true;
    }
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| {
            tcl_registry::dialects::TCL_SOURCE_EXTENSIONS.contains(&extension.as_str())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additions_choose_the_highest_minimum() {
        let reports = [
            report("json", "1.2"),
            report("json", "1.10"),
            report("http", "2.9"),
        ];
        let additions = collect_additions(&reports);
        assert_eq!(additions["json"].1, "1.10");
        assert_eq!(additions["http"].1, "2.9");
    }

    #[test]
    fn unsafe_manifest_atoms_are_rejected() {
        assert!(manifest_atom("json::write"));
        assert!(!manifest_atom("$package"));
        assert!(!manifest_atom("two words"));
        assert!(!manifest_atom("bad;command"));
    }

    fn report(name: &str, minimum: &str) -> RequirementReport {
        RequirementReport {
            name: Some(name.to_owned()),
            minimum: Some(minimum.to_owned()),
            expression: name.to_owned(),
            version_expression: Some(minimum.to_owned()),
            file: "main.tcl".to_owned(),
            line: 1,
            conditional: false,
            exact: false,
            resolution: "literal",
            status: "candidate",
            reason: None,
        }
    }
}
