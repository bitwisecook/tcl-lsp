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

//! Generate C Tcl release catalogues from pinned source trees and their binaries.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use tcl_dialect::TclVersion;
use tcl_test_support::{TclSourceTree, locate_source_tree, run_script_from_source_tree};

use crate::util::{format_rust, repo_root, write_if_changed};

#[path = "release_policies.rs"]
mod policies;

const DATA: &str = "rust/tcl-dialect/data/release-facts.tsv";
/// The pinned patchlevel and source tag per release: the one owner of those
/// two columns, which the rendered table carries for `TclVersion::patchlevel`.
const MANIFEST: &str = "rust/tcl-dialect/data/reference-toolchains.tsv";
const OUTPUT: &str = "rust/tcl-dialect/src/generated_release_facts.rs";
const PROBE: &str = include_str!("../../tcl-dialect/data/release-facts.tcl");

fn read(root: &Path, path: &str) -> Result<String> {
    let path = root.join(path);
    fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
}

fn table_names(source: &str, table: &str) -> Result<BTreeSet<String>> {
    let pattern = format!(r"(?s)\b{table}\s*\[\s*\]\s*=\s*\{{(.*?)\n\}};");
    let table = Regex::new(&pattern)?
        .captures(source)
        .with_context(|| format!("missing C table {table}"))?
        .get(1)
        .context("C table capture")?;
    Ok(Regex::new(r#"(?m)^\s*\{\s*"([^"]+)"#)?
        .captures_iter(table.as_str())
        .map(|row| row[1].to_owned())
        .collect())
}

fn source_math_functions(tree: &TclSourceTree, version: TclVersion) -> Result<BTreeSet<String>> {
    let (path, table) = if version == TclVersion::V8_4 {
        ("generic/tclExecute.c", "tclBuiltinFuncTable")
    } else {
        ("generic/tclBasic.c", "BuiltinFuncTable")
    };
    let mut names = table_names(&read(&tree.root, path)?, table)?;
    let init = read(&tree.root, "library/init.tcl")?;
    let scripted = Regex::new(r"(?ms)^    namespace eval mathfunc \{(.*?)^    \}")?;
    if let Some(block) = scripted.captures(&init) {
        names.extend(
            Regex::new(r"(?m)^\s*proc ([a-zA-Z0-9_]+) ")?
                .captures_iter(&block[1])
                .map(|row| row[1].to_owned()),
        );
    }
    Ok(names)
}

fn source_encodings(tree: &TclSourceTree) -> Result<BTreeSet<String>> {
    let directory = tree.library_dir().join("encoding");
    let mut names = BTreeSet::new();
    for entry in
        fs::read_dir(&directory).with_context(|| format!("read {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "enc") {
            names.insert(
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .context("encoding filename is not UTF-8")?
                    .to_owned(),
            );
        }
    }
    let source = read(&tree.root, "generic/tclEncoding.c")?;
    let builtin = Regex::new(r#"type\.encodingName\s*=\s*"([^"]+)"#)?;
    names.extend(builtin.captures_iter(&source).map(|row| row[1].to_owned()));
    ensure!(!names.is_empty(), "source encoding catalogue is empty");
    Ok(names)
}

fn bundled_packages(tree: &TclSourceTree) -> Result<Vec<(String, String)>> {
    let path = tree.root.join("pkgs/package.list.txt");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    for line in fs::read_to_string(path)?.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (directory, name) = line
            .split_once(char::is_whitespace)
            .context("package.list.txt row requires directory and name")?;
        let name = name.trim();
        let name = name
            .strip_prefix('{')
            .and_then(|name| name.strip_suffix('}'))
            .unwrap_or(name);
        ensure!(!name.is_empty(), "package.list.txt name is empty");
        rows.push((directory.to_owned(), name.to_owned()));
    }
    rows.sort();
    rows.dedup();
    Ok(rows)
}

#[derive(Default)]
struct Observation {
    math_functions: BTreeSet<String>,
    encodings: BTreeSet<String>,
    commands: BTreeSet<String>,
    provided: BTreeMap<String, (String, bool)>,
    tcloo: Option<(String, String)>,
}

fn observe(tree: &TclSourceTree, version: TclVersion) -> Result<Observation> {
    let output = run_script_from_source_tree(tree, version, PROBE.as_bytes())?
        .strict_text()
        .map_err(anyhow::Error::msg)?;
    let mut observation = Observation::default();
    let mut patchlevel = None;
    for line in output.lines() {
        let (kind, value) =
            tcl_test_support::fixture::tsv_split_once(line).context("invalid release probe row")?;
        match kind {
            "patchlevel" => patchlevel = Some(value),
            "math" => {
                observation.math_functions.insert(value.to_owned());
            }
            "encoding" => {
                observation.encodings.insert(value.to_owned());
            }
            "command" => {
                observation.commands.insert(value.to_owned());
            }
            "provided" => {
                let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(value).collect();
                ensure!(
                    fields.len() == 3 && matches!(fields[2], "0" | "1"),
                    "invalid package observation"
                );
                ensure!(
                    observation
                        .provided
                        .insert(
                            fields[0].to_owned(),
                            (fields[1].to_owned(), fields[2] == "1")
                        )
                        .is_none(),
                    "duplicate provided package"
                );
            }
            "oo" => {
                let (version, patchlevel) = tcl_test_support::fixture::tsv_split_once(value)
                    .context("TclOO version observation")?;
                ensure!(
                    observation
                        .tcloo
                        .replace((version.to_owned(), patchlevel.to_owned()))
                        .is_none(),
                    "duplicate TclOO observation"
                );
            }
            _ => bail!("unknown release probe row: {line}"),
        }
    }
    ensure!(
        patchlevel == Some(tree.patchlevel.as_str()),
        "release probe patchlevel mismatch"
    );
    ensure!(
        !observation.commands.is_empty(),
        "release probe command roster is empty"
    );
    ensure!(
        observation.math_functions == source_math_functions(tree, version)?,
        "Tcl {} source and oracle math functions disagree",
        tree.patchlevel
    );
    ensure!(
        observation.encodings == source_encodings(tree)?,
        "Tcl {} source and oracle encodings disagree",
        tree.patchlevel
    );
    check_tcloo_source(tree, observation.tcloo.as_ref())?;
    Ok(observation)
}

fn check_tcloo_source(tree: &TclSourceTree, observed: Option<&(String, String)>) -> Result<()> {
    let path = tree.root.join("generic/tclOO.h");
    if !path.exists() {
        ensure!(observed.is_none(), "TclOO observed without core source");
        return Ok(());
    }
    let source = fs::read_to_string(path)?;
    let definition = Regex::new(r"(?m)^#\s*define\s+(TCLOO_VERSION|TCLOO_PATCHLEVEL)\s+([^\n]+)")?;
    let mut fields = BTreeMap::new();
    let quoted = Regex::new(r#""([^"]*)""#)?;
    for row in definition.captures_iter(&source) {
        fields.insert(row[1].to_owned(), row[2].to_owned());
    }
    let version_expression = fields.get("TCLOO_VERSION").context("TCLOO_VERSION")?;
    let version = quoted
        .captures_iter(version_expression)
        .map(|row| row[1].to_owned())
        .collect::<String>();
    ensure!(!version.is_empty(), "empty source TclOO version");
    let patch_expression = fields.get("TCLOO_PATCHLEVEL").context("TCLOO_PATCHLEVEL")?;
    let mut patchlevel = if patch_expression.contains("TCLOO_VERSION") {
        version.clone()
    } else {
        String::new()
    };
    for row in quoted.captures_iter(patch_expression) {
        patchlevel.push_str(&row[1]);
    }
    ensure!(
        observed == Some(&(version, patchlevel)),
        "TclOO source and observed versions disagree"
    );
    Ok(())
}

fn extract(root: &Path) -> Result<String> {
    let manifest = read(root, MANIFEST)?;
    let mut text = String::from("# Tcl release\tfact kind\tvalue\toptional values\n");
    let lines: Vec<_> = manifest
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    ensure!(
        lines.len() == TclVersion::ALL.len(),
        "missing release manifest row"
    );
    for (count, line) in lines.into_iter().enumerate() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(line).collect();
        ensure!(
            fields.len() == 3,
            "reference-toolchain row requires three columns"
        );
        let version = TclVersion::from_version_string(fields[0]).context("unknown Tcl release")?;
        ensure!(
            TclVersion::ALL.get(count) == Some(&version),
            "release manifest is not in ladder order"
        );
        let tree = locate_source_tree(root, version, None)?.with_context(|| {
            format!(
                "missing Tcl {} source tree; run make fetch-tcl-source",
                fields[0]
            )
        })?;
        ensure!(
            tree.patchlevel == fields[1],
            "source tree patchlevel differs from pinned manifest"
        );
        let observation = observe(&tree, version)?;
        writeln!(text, "{}\tmeta\t{}\t{}", fields[0], fields[1], fields[2])?;
        for (kind, names) in [
            ("math", observation.math_functions),
            ("encoding", observation.encodings),
            ("command", observation.commands),
        ] {
            for name in names {
                writeln!(text, "{}\t{kind}\t{name}", fields[0])?;
            }
        }
        for (name, (version, stub)) in observation.provided {
            writeln!(
                text,
                "{}\tprovided\t{name}\t{version}\t{}",
                fields[0],
                u8::from(stub)
            )?;
        }
        if let Some((version, patchlevel)) = observation.tcloo {
            writeln!(text, "{}\too\t{version}\t{patchlevel}", fields[0])?;
        }
        for (directory, name) in bundled_packages(&tree)? {
            writeln!(text, "{}\tpackage\t{directory}\t{name}", fields[0])?;
        }
    }
    Ok(text)
}

fn fact_rows(data: &str) -> Result<Vec<Vec<&str>>> {
    let rows: Vec<_> = data
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| tcl_test_support::fixture::tsv_fields(line).collect::<Vec<_>>())
        .collect();
    for row in &rows {
        ensure!(
            matches!(row.len(), 3..=5),
            "release facts require three to five fields"
        );
        ensure!(
            TclVersion::from_version_string(row[0]).is_some(),
            "unknown release in catalogue"
        );
        ensure!(
            matches!(
                row[1],
                "meta" | "math" | "encoding" | "command" | "package" | "provided" | "oo"
            ),
            "unknown release fact kind"
        );
        ensure!(
            row.len()
                == match row[1] {
                    "meta" | "package" | "oo" => 4,
                    "provided" => 5,
                    _ => 3,
                },
            "invalid release fact width"
        );
    }
    Ok(rows)
}

/// The manifest's `(patchlevel, source tag)` per release, in ladder order.
fn manifest_rows(manifest: &str) -> Result<Vec<(&str, &str)>> {
    let rows: Vec<_> = manifest
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(line).collect();
            ensure!(
                fields.len() == 3 && fields.iter().all(|field| !field.is_empty()),
                "reference-toolchain row requires three non-empty columns"
            );
            Ok((fields[0], (fields[1], fields[2])))
        })
        .collect::<Result<_>>()?;
    ensure!(
        rows.iter().map(|(release, _)| *release).eq(TclVersion::ALL
            .iter()
            .map(|version| version.version_string())),
        "reference manifest is not one row per release in ladder order"
    );
    Ok(rows.into_iter().map(|(_, pins)| pins).collect())
}

fn render(data: &str, manifest: &str) -> Result<String> {
    let pins = manifest_rows(manifest)?;
    let mut text = String::from("// @generated by cargo xtask release-facts; do not edit.\n");
    text.push_str("pub(crate) const RELEASE_FACTS: &[ReleaseFacts] = &[\n");
    let rows = fact_rows(data)?;
    for (version, (patchlevel, source_tag)) in TclVersion::ALL.into_iter().zip(pins) {
        let release = version.version_string();
        let release_rows: Vec<_> = rows.iter().filter(|row| row[0] == release).collect();
        let metadata: Vec<_> = release_rows.iter().filter(|row| row[1] == "meta").collect();
        ensure!(
            metadata.len() == 1,
            "{release} requires exactly one metadata row"
        );
        let metadata = metadata[0];
        ensure!(
            metadata[2] == patchlevel && metadata[3] == source_tag,
            "{release} catalogue disagrees with reference manifest"
        );
        let profile_name = format!("tcl{release}");
        writeln!(
            text,
            "ReleaseFacts {{ version: TclVersion::{version:?}, native_c_version: tcl_core_types::NativeCVersion::{version:?}, version_string: {release:?}, profile_name: {:?}, patchlevel: {:?}, source_tag: {:?},",
            profile_name, metadata[2], metadata[3]
        )?;
        for (field, kind) in [
            ("math_functions", "math"),
            ("encodings", "encoding"),
            ("commands", "command"),
        ] {
            let names: Vec<_> = release_rows
                .iter()
                .filter(|row| row[1] == kind)
                .map(|row| row[2])
                .collect();
            ensure!(
                !names.is_empty() && names.windows(2).all(|pair| pair[0] < pair[1]),
                "{release} {kind} facts must be non-empty, sorted and unique"
            );
            writeln!(text, "{field}: &[")?;
            for name in names {
                writeln!(text, "{name:?},")?;
            }
            text.push_str("],\n");
        }
        render_provisions(&release_rows, &mut text)?;
        text.push_str("bundled_packages: &[\n");
        for row in release_rows.iter().filter(|row| row[1] == "package") {
            writeln!(text, "({:?}, {:?}),", row[2], row[3])?;
        }
        text.push_str("],\n},\n");
    }
    text.push_str("];\n");
    render_functions(&rows, &mut text)?;
    render_command_surfaces(&rows, &mut text)?;
    format_rust(&text)
}

fn render_provisions(rows: &[&Vec<&str>], text: &mut String) -> Result<()> {
    let provided: Vec<_> = rows.iter().filter(|row| row[1] == "provided").collect();
    ensure!(
        !provided.is_empty() && provided.windows(2).all(|pair| pair[0][2] < pair[1][2]),
        "provided package rows must be non-empty, sorted and unique"
    );
    text.push_str("provided_packages: &[\n");
    for row in &provided {
        ensure!(matches!(row[4], "0" | "1"), "invalid package ifneeded flag");
        writeln!(text, "({:?}, {:?}, {}),", row[2], row[3], row[4] == "1")?;
    }
    text.push_str("], core_packages: &[\n");
    // Semantic package subjects select the existing engine-supported subset;
    // their presence, version and ifneeded facts come solely from observations.
    for (role, names) in [
        ("InterpreterRelease", ["Tcl", "tcl"]),
        ("ObjectSystem", ["TclOO", "tcl::oo"]),
    ] {
        for name in names {
            if let Some(row) = provided.iter().find(|row| row[2] == name) {
                writeln!(
                    text,
                    "crate::CorePackage {{ role: crate::CorePackageRole::{role}, name: {:?}, version: {:?}, ifneeded_stub: {} }},",
                    row[2],
                    row[3],
                    row[4] == "1"
                )?;
            }
        }
    }
    text.push_str("],\n");
    let versions: Vec<_> = rows.iter().filter(|row| row[1] == "oo").collect();
    ensure!(versions.len() <= 1, "duplicate TclOO version row");
    if let Some(row) = versions.first() {
        writeln!(text, "tcloo_version: Some(({:?}, {:?})),", row[2], row[3])?;
    } else {
        text.push_str("tcloo_version: None,\n");
    }
    Ok(())
}

fn render_functions(rows: &[Vec<&str>], text: &mut String) -> Result<()> {
    let mut functions = BTreeMap::new();
    for version in TclVersion::ALL {
        for row in rows
            .iter()
            .filter(|row| row[0] == version.version_string() && row[1] == "math")
        {
            functions.entry(row[2]).or_insert(version);
        }
    }
    text.push_str("pub(crate) const TCL_MATHFUNCS: &[crate::model::MathFunc] = &[\n");
    for (name, version) in functions {
        let release = version.version_string().replace('.', "_");
        writeln!(
            text,
            "crate::model::MathFunc {{ name: {name:?}, since: crate::model::Release::TCL_{release}, needs_math_extension: false }},"
        )?;
    }
    text.push_str("];\n");
    Ok(())
}

fn render_command_surfaces(rows: &[Vec<&str>], text: &mut String) -> Result<()> {
    let commands: BTreeSet<_> = rows
        .iter()
        .filter(|row| row[1] == "command")
        .map(|row| row[2])
        .collect();
    let mut aliases = BTreeMap::new();
    for command in commands {
        let registry_name = tcl_syntax::naming::unroot_rooted_key(command)
            .context("reference command must be root-qualified")?;
        for name in [command, registry_name] {
            ensure!(
                aliases.insert(name, command).is_none(),
                "duplicate reference command alias {name:?}"
            );
        }
    }
    text.push_str("pub(crate) const REFERENCE_COMMAND_SURFACES: &[(&str, &[crate::model::SpecSurface])] = &[\n");
    for (name, command) in aliases {
        let presence: Vec<_> = TclVersion::ALL
            .iter()
            .map(|version| {
                rows.iter().any(|row| {
                    row[0] == version.version_string() && row[1] == "command" && row[2] == command
                })
            })
            .collect();
        writeln!(
            text,
            "({name:?}, &[crate::model::SpecSurface::core_in(crate::model::Family::Tcl, &["
        )?;
        let mut start = None;
        for (index, present) in presence
            .into_iter()
            .chain(std::iter::once(false))
            .enumerate()
        {
            if present && start.is_none() {
                start = Some(index);
            }
            if !present && let Some(begin) = start.take() {
                let lower = TclVersion::ALL[begin].version_string();
                let upper = TclVersion::ALL
                    .get(index)
                    .map_or("9.2", |version| version.version_string());
                writeln!(text, "({lower:?}, Some({upper:?})),")?;
            }
        }
        text.push_str("])]),\n");
    }
    text.push_str("];\n");
    Ok(())
}

pub fn run(check: bool, verify_sources: bool) -> Result<ExitCode> {
    policies::run(check, verify_sources)?;
    let root = repo_root();
    if verify_sources {
        let measured = extract(&root)?;
        if check {
            ensure!(
                read(&root, DATA)? == measured,
                "{DATA} differs from the pinned sources and oracles"
            );
        } else {
            write_if_changed(&root.join(DATA), measured)?;
        }
    }
    let rendered = render(&read(&root, DATA)?, &read(&root, MANIFEST)?)?;
    let path = root.join(OUTPUT);
    if check {
        let current = fs::read_to_string(&path).with_context(|| format!("read {OUTPUT}"))?;
        ensure!(
            current == rendered,
            "{OUTPUT} is stale; run cargo xtask release-facts"
        );
    } else {
        write_if_changed(&path, rendered)?;
    }
    eprintln!("release-facts: generated catalogues agree with extracted reference facts");
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_table_reader_handles_nested_initialisers_and_rejects_missing_tables() {
        let source = "BuiltinFunc sample[] = {\n{\"acos\", 1, {TCL_DOUBLE}, f},\n{\"abs\", 1, {TCL_EITHER}, f},\n{NULL, 0}\n};";
        assert_eq!(
            table_names(source, "sample").unwrap(),
            BTreeSet::from(["abs".to_owned(), "acos".to_owned()])
        );
        assert!(table_names(source, "absent").is_err());
    }

    #[test]
    fn offline_renderer_rejects_missing_metadata() {
        let data = include_str!("../../tcl-dialect/data/release-facts.tsv");
        let manifest = include_str!("../../tcl-dialect/data/reference-toolchains.tsv");
        let metadata = data
            .lines()
            .find(|line| line.starts_with("8.4\tmeta\t"))
            .unwrap();
        let altered = data.replace(metadata, "");
        assert!(
            render(&altered, manifest)
                .unwrap_err()
                .to_string()
                .contains("metadata")
        );
    }

    #[test]
    fn offline_renderer_rejects_duplicate_observations_and_unknown_kinds() {
        let data = include_str!("../../tcl-dialect/data/release-facts.tsv");
        let manifest = include_str!("../../tcl-dialect/data/reference-toolchains.tsv");
        let duplicated = data.replace("8.4\tmath\tabs\n", "8.4\tmath\tabs\n8.4\tmath\tabs\n");
        assert!(
            render(&duplicated, manifest)
                .unwrap_err()
                .to_string()
                .contains("unique")
        );
        let unknown = data.replace("8.4\tmath\tabs\n", "8.4\tinvented\tabs\n");
        assert!(
            render(&unknown, manifest)
                .unwrap_err()
                .to_string()
                .contains("kind")
        );
    }

    #[test]
    fn offline_renderer_rejects_reference_manifest_drift() {
        let data = include_str!("../../tcl-dialect/data/release-facts.tsv");
        let manifest = include_str!("../../tcl-dialect/data/reference-toolchains.tsv");
        let altered = data.replace("8.4\tmeta\t8.4.20", "8.4\tmeta\t8.4.19");
        assert!(
            render(&altered, manifest)
                .unwrap_err()
                .to_string()
                .contains("manifest")
        );
        let moved = manifest.replace("8.4.20", "8.4.19");
        assert!(
            render(data, &moved)
                .unwrap_err()
                .to_string()
                .contains("manifest")
        );
    }
}
