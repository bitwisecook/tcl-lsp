// tcl-lsp — a Tcl language server and toolchain
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compare the registry's iRules surface with a local BIG-IP extract.
//!
//! The extract is an oracle input, not a second registry.  The command checks
//! exact command/event/profile names that the extract declares, and reports
//! the wider registry surface separately because a BIG-IP release can contain
//! commands not present in an older schema split.  Man-page SYNOPSIS literals
//! are collected as an advisory subcommand/option coverage measure; their
//! prose also contains enum values and placeholders, so those observations do
//! not become hard failures without a stronger schema signal.
//!
//! This is deliberately an on-demand gate: the proprietary/local BIG-IP
//! extract is not part of the repository or CI checkout.  Run it with
//! `cargo xtask registry-oracle --irules-root /path/to/bigip-extract`; pass
//! `--check --output ...` to verify a checked-in or CI-produced report.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tcl_dialect::model::{Family, SurfaceQuery, surface_admits};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::Value;

const DIALECT: &str = "f5-irules";

#[derive(Debug, Default)]
struct SourceSurface {
    commands: BTreeSet<String>,
    events: BTreeSet<String>,
    profiles: BTreeSet<String>,
    namespaces: BTreeSet<String>,
    artifact: String,
}

#[derive(Debug, Default)]
struct ManCoverage {
    pages: usize,
    pages_with_registry_command: usize,
    synopsis_literals: usize,
    unmodelled_literals: BTreeSet<String>,
}

/// Run the source-oracle audit, optionally writing or checking a report.
pub fn run(irules_root: &Path, output: Option<&Path>, check: bool) -> Result<ExitCode> {
    let source = SourceSurface::load(irules_root)?;
    // The one dialect name this audit accepts, through the ingress seam:
    // the `f5-irules` environment's registry generation, whose store is
    // the very `Arc` `registry_for_dialect` published.
    let registry = crate::environment::store_for_dialect(DIALECT);
    let events = tcl_registry::events::EventRegistry::build();
    let profiles = tcl_registry::profiles::ProfileRegistry::build();
    let report = render_report(
        irules_root,
        &source,
        registry,
        &events,
        &profiles,
        &mut ManCoverage::default(),
    )?;

    let hard_failures = hard_failures(&source, registry, &events, &profiles);
    if !hard_failures.is_empty() {
        eprintln!(
            "registry-oracle: {} hard coverage failure(s):",
            hard_failures.len()
        );
        for failure in &hard_failures {
            eprintln!("  - {failure}");
        }
    }

    if let Some(path) = output {
        if check {
            let existing = fs::read_to_string(path)
                .with_context(|| format!("reading oracle report {}", path.display()))?;
            if existing != report {
                bail!(
                    "{} is stale — rerun `cargo xtask registry-oracle --irules-root {} --output {}`",
                    path.display(),
                    irules_root.display(),
                    path.display()
                );
            }
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating report directory {}", parent.display()))?;
            }
            fs::write(path, &report)
                .with_context(|| format!("writing oracle report {}", path.display()))?;
        }
    } else {
        print!("{report}");
    }

    if hard_failures.is_empty() {
        eprintln!("OK: registry source-oracle coverage is aligned for {DIALECT}.");
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

impl SourceSurface {
    fn load(root: &Path) -> Result<Self> {
        let split = root.join("irule-schema-split");
        if !split.is_dir() {
            bail!("missing iRules schema split directory: {}", split.display());
        }
        let mut out = Self {
            artifact: find_artifact_name(root),
            ..Self::default()
        };
        let global_commands = split.join("global_commands.json");
        if !global_commands.is_file() {
            bail!(
                "missing iRules global command source: {}",
                global_commands.display()
            );
        }
        let mut command_files = vec![global_commands];
        let mut namespaces: Vec<PathBuf> = fs::read_dir(&split)
            .with_context(|| format!("reading {}", split.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("namespace_")
                            && Path::new(name)
                                .extension()
                                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
                    })
            })
            .collect();
        namespaces.sort();
        command_files.extend(namespaces);
        for path in command_files {
            if !path.exists() {
                continue;
            }
            for entry in read_array(&path)? {
                if let Some(name) = entry.get("commandName").and_then(Value::as_str)
                    && !name.is_empty()
                {
                    out.commands.insert(name.to_owned());
                }
            }
        }

        let events_path = split.join("events.json");
        for entry in read_array(&events_path)? {
            if let Some(name) = entry.get("eventName").and_then(Value::as_str)
                && !name.is_empty()
            {
                out.events.insert(name.to_owned());
            }
            if let Some(values) = entry.get("profiles:").and_then(Value::as_array) {
                for value in values {
                    if let Some(name) = value.as_str() {
                        out.profiles.insert(name.to_owned());
                    }
                }
            }
        }

        let namespaces_path = split.join("namespaces.json");
        for value in read_array(&namespaces_path)? {
            if let Some(name) = value.as_str() {
                out.namespaces.insert(name.to_owned());
            }
        }
        if out.commands.is_empty() {
            bail!(
                "iRules schema contains no commands under {}",
                split.display()
            );
        }
        Ok(out)
    }
}

fn read_array(path: &Path) -> Result<Vec<Value>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn find_artifact_name(root: &Path) -> String {
    let mut names: Vec<String> = fs::read_dir(root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (name.starts_with("irule-schema-")
                && Path::new(&name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json")))
            .then_some(name)
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .next()
        .unwrap_or_else(|| "unknown".to_owned())
}

fn hard_failures(
    source: &SourceSurface,
    registry: &tcl_registry::CommandRegistry,
    events: &tcl_registry::events::EventRegistry,
    profiles: &tcl_registry::profiles::ProfileRegistry,
) -> Vec<String> {
    let registry_commands = irule_command_names(registry);
    let missing_commands: Vec<&str> = source
        .commands
        .iter()
        .map(String::as_str)
        .filter(|name| !registry_commands.contains(name))
        .collect();
    let missing_events: Vec<&str> = source
        .events
        .iter()
        .map(String::as_str)
        .filter(|name| !events.is_known(name))
        .collect();
    let missing_profiles: Vec<&str> = source
        .profiles
        .iter()
        .map(String::as_str)
        .filter(|name| profiles.get_profile(name).is_none())
        .collect();
    let mut out = Vec::new();
    if !missing_commands.is_empty() {
        out.push(format!(
            "{} source command(s) absent from the registry: {}",
            missing_commands.len(),
            missing_commands.join(", ")
        ));
    }
    if !missing_events.is_empty() {
        out.push(format!(
            "{} source event(s) absent from the registry: {}",
            missing_events.len(),
            missing_events.join(", ")
        ));
    }
    if !missing_profiles.is_empty() {
        out.push(format!(
            "{} source profile(s) absent from the registry: {}",
            missing_profiles.len(),
            missing_profiles.join(", ")
        ));
    }
    out
}

fn render_report(
    root: &Path,
    source: &SourceSurface,
    registry: &tcl_registry::CommandRegistry,
    events: &tcl_registry::events::EventRegistry,
    profiles: &tcl_registry::profiles::ProfileRegistry,
    coverage: &mut ManCoverage,
) -> Result<String> {
    let source_commands: BTreeSet<&str> = source.commands.iter().map(String::as_str).collect();
    let registry_commands = irule_command_names(registry);
    let source_events: BTreeSet<&str> = source.events.iter().map(String::as_str).collect();
    let registry_events: BTreeSet<&str> = events.all_event_names().into_iter().collect();
    let source_profiles: BTreeSet<&str> = source.profiles.iter().map(String::as_str).collect();
    let registry_profiles: BTreeSet<&str> = profiles.all_profile_names().into_iter().collect();
    let registry_namespaces: BTreeSet<&str> =
        profiles.all_namespace_prefixes().into_iter().collect();
    let source_namespaces: BTreeSet<&str> = source.namespaces.iter().map(String::as_str).collect();
    *coverage = man_coverage(root, registry)?;

    let mut out = String::new();
    out.push_str("# Registry source oracle\n\n");
    out.push_str("> Generated by `cargo xtask registry-oracle`; do not edit by hand.\n\n");
    let _ = writeln!(out, "Source artifact: `{}`\n", source.artifact);
    out.push_str("## Exact schema coverage\n\n");
    out.push_str(
        "| Surface | Extract | Registry | Extract-only | Registry-only |\n|---|---:|---:|---:|---:|\n",
    );
    write_row(&mut out, "commands", &source_commands, &registry_commands);
    write_row(&mut out, "events", &source_events, &registry_events);
    write_row(&mut out, "profiles", &source_profiles, &registry_profiles);
    write_row(
        &mut out,
        "namespaces",
        &source_namespaces,
        &registry_namespaces,
    );
    out.push_str("\nHard failures are extract-only commands, events, or profiles. Registry-only entries are retained in the report because the registry may intentionally cover a newer BIG-IP release than the supplied extract. Namespace metadata is reported as a coverage signal; a namespace may be present in the command surface without a profile requirement.\n\n");
    out.push_str("## Man-page synopsis coverage\n\n");
    let _ = writeln!(
        out,
        "Parsed `{}` man pages; `{}` resolved to registry commands. Extracted `{}` quoted SYNOPSIS literals. `{}` literals are not currently represented as a declared subcommand or option (this is advisory because SYNOPSIS literals also include enum values and placeholders).",
        coverage.pages,
        coverage.pages_with_registry_command,
        coverage.synopsis_literals,
        coverage.unmodelled_literals.len()
    );
    if !coverage.unmodelled_literals.is_empty() {
        let sample: Vec<&str> = coverage
            .unmodelled_literals
            .iter()
            .take(40)
            .map(String::as_str)
            .collect();
        let _ = writeln!(
            out,
            "\nUnmodelled literal sample: `{}`",
            sample.join("`, `")
        );
    }
    let root_label = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("external-extract");
    let _ = writeln!(out, "\nSource root: `{root_label}`");
    Ok(out)
}

fn write_row(out: &mut String, label: &str, extract: &BTreeSet<&str>, registry: &BTreeSet<&str>) {
    let extract_only = extract.difference(registry).count();
    let registry_only = registry.difference(extract).count();
    let _ = writeln!(
        out,
        "| {label} | {} | {} | {extract_only} | {registry_only} |",
        extract.len(),
        registry.len()
    );
}

fn irule_command_names(registry: &tcl_registry::CommandRegistry) -> BTreeSet<&str> {
    registry
        .command_names()
        .filter(|name| {
            registry
                .get(name)
                .and_then(|spec| spec.surface)
                .is_some_and(|rows| {
                    surface_admits(rows, Some(&SurfaceQuery::any_release(Family::F5Irules)))
                })
        })
        .collect()
}

fn man_coverage(root: &Path, registry: &tcl_registry::CommandRegistry) -> Result<ManCoverage> {
    let Some(man_dir) = fs::read_dir(root)
        .with_context(|| format!("reading {}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("man-"))
        })
        .min()
    else {
        return Ok(ManCoverage::default());
    };
    let mut out = ManCoverage::default();
    let quoted = Regex::new(r"'([^']+)'")?;
    for entry in fs::read_dir(&man_dir).with_context(|| format!("reading {}", man_dir.display()))? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.starts_with("ltm_rule_command_") || !name.ends_with(".3") {
            continue;
        }
        out.pages += 1;
        let text = fs::read_to_string(&path).unwrap_or_default();
        let command = man_command_name(&text, registry);
        let Some(command) = command else { continue };
        out.pages_with_registry_command += 1;
        let Some(spec) = registry.get(&command) else {
            continue;
        };
        let known = declared_words(spec);
        for section_line in synopsis_lines(&text) {
            for capture in quoted.captures_iter(section_line) {
                let literal = capture[1].replace(r"\-", "-");
                out.synopsis_literals += 1;
                if !known.contains(literal.as_str()) {
                    out.unmodelled_literals.insert(literal);
                }
            }
        }
    }
    Ok(out)
}

fn man_command_name(text: &str, registry: &tcl_registry::CommandRegistry) -> Option<String> {
    text.lines()
        .filter_map(|line| line.strip_prefix(".SH "))
        .map(|line| line.trim_matches('"').replace(r"\-", "-"))
        .find(|candidate| registry.get(candidate).is_some())
}

fn synopsis_lines(text: &str) -> impl Iterator<Item = &str> {
    let mut in_synopsis = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        if let Some(section) = line.strip_prefix(".SH ") {
            in_synopsis = section.trim_matches('"') == "SYNOPSIS";
            continue;
        }
        if in_synopsis {
            lines.push(line);
        }
    }
    lines.into_iter()
}

fn declared_words(spec: &tcl_registry::spec::CommandSpec) -> BTreeSet<&str> {
    let mut out: BTreeSet<&str> = spec.subcommands.iter().map(|sub| sub.name).collect();
    out.extend(spec.options.iter().map(|option| option.name));
    for sub in spec.subcommands {
        out.extend(sub.options.iter().map(|option| option.name));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_surface_reads_schema_sets() {
        let root = std::env::temp_dir().join(format!("tcl-lsp-oracle-test-{}", std::process::id()));
        let split = root.join("irule-schema-split");
        fs::create_dir_all(&split).expect("create schema split");
        fs::write(
            split.join("global_commands.json"),
            r#"[{"commandName":"HTTP::header"}]"#,
        )
        .expect("write commands");
        fs::write(
            split.join("events.json"),
            r#"[{"eventName":"HTTP_REQUEST","profiles:":["HTTP"]}]"#,
        )
        .expect("write events");
        fs::write(split.join("namespaces.json"), r#"["HTTP"]"#).expect("write namespaces");
        let surface = SourceSurface::load(&root).expect("load surface");
        assert!(surface.commands.contains("HTTP::header"));
        assert!(surface.events.contains("HTTP_REQUEST"));
        assert!(surface.profiles.contains("HTTP"));
        assert!(surface.namespaces.contains("HTTP"));
        let _ = fs::remove_dir_all(root);
    }
}
