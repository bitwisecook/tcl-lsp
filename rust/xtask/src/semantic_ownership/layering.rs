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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

const PROTECTED: &[&str] = &[
    "tcl-compiler",
    "tcl-codegen",
    "tcl-lsp-core",
    "tcl-lsp-db",
    "tcl-bigip-schema",
    "tcl-source",
];

#[derive(Deserialize)]
struct Config {
    bans: Bans,
}
#[derive(Deserialize)]
struct Bans {
    deny: Vec<Ban>,
}
#[derive(Deserialize)]
struct Ban {
    name: String,
    #[serde(default)]
    wrappers: BTreeSet<String>,
}
#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}
#[derive(Deserialize)]
struct Package {
    name: String,
    dependencies: Vec<Dependency>,
}
#[derive(Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
}

fn violations(packages: &[Package], policy: &BTreeMap<&str, &BTreeSet<String>>) -> Vec<String> {
    let mut violations = Vec::new();
    for package in packages {
        let allowed: Option<&[&str]> = match package.name.as_str() {
            "tcl-bigip-schema" => Some(&["tcl-dialect"]),
            "tcl-source" => Some(&["tcl-core-types", "tcl-dialect", "tcl-lexer", "tcl-syntax"]),
            _ => None,
        };
        for dependency in &package.dependencies {
            if allowed.is_some_and(|owners| {
                !(owners.contains(&dependency.name.as_str())
                    || (package.name == "tcl-source"
                        && dependency.kind.as_deref() == Some("dev")
                        && dependency.name == "tcl-test-support"))
                    && packages
                        .iter()
                        .any(|candidate| candidate.name == dependency.name)
            }) {
                violations.push(format!(
                    "{} -> {} is outside its foundation owners",
                    package.name, dependency.name
                ));
            }
            if policy
                .get(dependency.name.as_str())
                .is_some_and(|wrappers| !wrappers.contains(&package.name))
            {
                violations.push(format!(
                    "{} -> {} is outside the declared layers",
                    package.name, dependency.name
                ));
            }
        }
    }
    violations.sort_unstable();
    violations.dedup();
    violations
}

pub(super) fn run(root: &Path) -> Result<ExitCode> {
    let config: Config = toml::from_str(&fs::read_to_string(root.join("deny.toml"))?)?;
    let mut policy = BTreeMap::new();
    for ban in &config.bans.deny {
        if PROTECTED.contains(&ban.name.as_str()) {
            ensure!(
                !ban.wrappers.is_empty(),
                "{} needs cargo-deny wrappers",
                ban.name
            );
            ensure!(
                policy.insert(ban.name.as_str(), &ban.wrappers).is_none(),
                "duplicate crate layer for {}",
                ban.name
            );
        }
    }
    ensure!(
        policy.len() == PROTECTED.len(),
        "deny.toml is missing protected crate layers"
    );

    let mut errors = Vec::new();
    for manifest in ["Cargo.toml", "runtime/rust/Cargo.toml"] {
        let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .current_dir(root)
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--locked",
                "--manifest-path",
                manifest,
            ])
            .output()
            .with_context(|| format!("read Cargo dependency graph for {manifest}"))?;
        ensure!(
            output.status.success(),
            "{manifest}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let graph: Metadata = serde_json::from_slice(&output.stdout)?;
        errors.extend(violations(&graph.packages, &policy));
    }
    for error in &errors {
        eprintln!("crate-layer: {error}");
    }
    if errors.is_empty() {
        println!("crate-layer: OK ({} protected crates)", policy.len());
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_upward_edge_is_rejected_even_through_a_dependency_alias() {
        let graph = r#"{"packages":[
            {"name":"tcl-lexer","dependencies":[{"name":"tcl-lsp-core","rename":"formatter"}]},
            {"name":"tcl-lsp-server","dependencies":[{"name":"tcl-lsp-core"}]}
        ]}"#;
        let metadata: Metadata = serde_json::from_str(graph).unwrap();
        let wrappers = BTreeSet::from(["tcl-lsp-server".to_owned()]);
        let policy = BTreeMap::from([("tcl-lsp-core", &wrappers)]);
        assert_eq!(
            violations(&metadata.packages, &policy),
            ["tcl-lexer -> tcl-lsp-core is outside the declared layers"]
        );
    }
    #[test]
    fn source_and_schema_keep_their_foundation_boundary() {
        let graph = r#"{"packages":[
            {"name":"tcl-source","dependencies":[{"name":"tcl-registry"}]},
            {"name":"tcl-bigip-schema","dependencies":[{"name":"tcl-registry"}]},
            {"name":"tcl-registry","dependencies":[]}
        ]}"#;
        let metadata: Metadata = serde_json::from_str(graph).unwrap();
        assert_eq!(
            violations(&metadata.packages, &BTreeMap::new()),
            [
                "tcl-bigip-schema -> tcl-registry is outside its foundation owners",
                "tcl-source -> tcl-registry is outside its foundation owners",
            ]
        );
    }
}
