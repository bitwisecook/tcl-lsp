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

//! Source-data maintenance for the optional tcl-sslictcl crate.
//!
//! The data crate owns source-specific importers and generated TLS data.
//! This task owns the repository contract around that data: update is the only
//! network-capable operation, while check is deliberately offline and verifies
//! the provenance manifest, file hashes, paths, and (when supplied) the
//! generator's deterministic check hook.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::util::repo_root;

const DATA_CRATE: &str = "rust/tcl-sslictcl";
const DATA_DIR: &str = "rust/tcl-sslictcl/data";
const RAW_DIR: &str = "rust/tcl-sslictcl/data/raw";
const GENERATED_DIR: &str = "rust/tcl-sslictcl/data/generated";
const PROVENANCE: &str = "rust/tcl-sslictcl/data/provenance.json";

/// Run the source-data operation selected by the CLI.
pub fn run(operation: &str, max_age_days: Option<u64>) -> Result<ExitCode> {
    let root = repo_root();
    let crate_dir = root.join(DATA_CRATE);
    if !crate_dir.is_dir() {
        println!(
            "sslictcl-data: {DATA_CRATE}/ is not present; source-data gate is inactive until the data crate lands"
        );
        return Ok(ExitCode::SUCCESS);
    }
    if !root.join(DATA_DIR).is_dir()
        || !root.join(RAW_DIR).is_dir()
        || !root.join(GENERATED_DIR).is_dir()
        || !root.join(PROVENANCE).is_file()
    {
        println!(
            "sslictcl-data: {DATA_DIR}/ is not present; source-data gate is inactive until the data bundle lands"
        );
        return Ok(ExitCode::SUCCESS);
    }

    match operation {
        "update" => update(&root),
        "check" => check(&root, max_age_days),
        _ => bail!("unknown sslictcl-data operation {operation:?}; expected update or check"),
    }
}

fn update(root: &Path) -> Result<ExitCode> {
    let hook = find_hook(
        root,
        &[
            "rust/tcl-sslictcl/scripts/update-source-data.sh",
            "rust/tcl-sslictcl/scripts/update-data.sh",
            "rust/tcl-sslictcl/data/update.sh",
        ],
    )
    .ok_or_else(|| {
        anyhow::anyhow!(
            "no SslicTcl source-data updater found; add rust/tcl-sslictcl/scripts/update-source-data.sh"
        )
    })?;

    println!("sslictcl-data: running {}", display_relative(root, &hook));
    let status = Command::new("bash")
        .arg(&hook)
        .current_dir(root)
        .status()
        .with_context(|| format!("running {}", hook.display()))?;
    if !status.success() {
        bail!(
            "SslicTcl source-data updater exited with {}",
            status
                .code()
                .map_or_else(|| "a signal".to_owned(), |code| code.to_string())
        );
    }

    // Updating is allowed to use the network, but the resulting tree must be
    // suitable for the same offline gate that CI and release preparation use.
    check(root, None)
}

fn check(root: &Path, max_age_days: Option<u64>) -> Result<ExitCode> {
    let data_dir = root.join(DATA_DIR);
    let raw_dir = root.join(RAW_DIR);
    let generated_dir = root.join(GENERATED_DIR);
    let manifest_path = root.join(PROVENANCE);

    for (label, path) in [
        ("data", &data_dir),
        ("raw data", &raw_dir),
        ("generated data", &generated_dir),
    ] {
        if !path.is_dir() {
            bail!(
                "SslicTcl {label} directory is missing: {}",
                display_relative(root, path)
            );
        }
    }
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", display_relative(root, &manifest_path)))?;
    let manifest: Value = serde_json::from_str(&manifest_text)
        .with_context(|| format!("parsing {} as JSON", display_relative(root, &manifest_path)))?;
    validate_manifest(root, &manifest, max_age_days)?;

    if let Some(generator) = find_hook(
        root,
        &[
            "rust/tcl-sslictcl/scripts/generate-source-data.sh",
            "rust/tcl-sslictcl/scripts/generate-data.sh",
            "rust/tcl-sslictcl/data/generate.sh",
        ],
    ) {
        println!(
            "sslictcl-data: checking deterministic generated output with {}",
            display_relative(root, &generator)
        );
        let status = Command::new("bash")
            .arg(&generator)
            .arg("--check")
            .current_dir(root)
            .status()
            .with_context(|| format!("running {}", generator.display()))?;
        if !status.success() {
            bail!("SslicTcl generated-data check failed");
        }
    } else {
        println!(
            "sslictcl-data: no generator check hook; provenance hashes provide the structural offline check"
        );
    }

    println!("sslictcl-data: source data is valid and offline-readable");
    Ok(ExitCode::SUCCESS)
}

fn validate_manifest(root: &Path, manifest: &Value, max_age_days: Option<u64>) -> Result<()> {
    let object = manifest
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("provenance manifest must be a JSON object"))?;
    validate_schema(object)?;

    let sources = object
        .get("sources")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("provenance manifest needs a sources array"))?;
    if sources.is_empty() {
        bail!("provenance manifest sources array is empty");
    }
    let mut oldest_retrieval: Option<DateTime<Utc>> = None;
    for (index, source) in sources.iter().enumerate() {
        let source = source
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("sources[{index}] must be an object"))?;
        for field in ["id", "url", "revision", "retrieved_at", "license"] {
            let value = source.get(field).and_then(Value::as_str).unwrap_or("");
            if value.is_empty() {
                bail!("sources[{index}] is missing non-empty {field}");
            }
        }
        let retrieved = source["retrieved_at"]
            .as_str()
            .expect("retrieved_at was checked above")
            .parse::<DateTime<Utc>>()
            .with_context(|| format!("sources[{index}].retrieved_at is not RFC3339"))?;
        oldest_retrieval = Some(oldest_retrieval.map_or(retrieved, |old| old.min(retrieved)));
    }
    if let Some(max_age_days) = max_age_days {
        let oldest = oldest_retrieval.expect("sources is non-empty");
        let age = Utc::now().signed_duration_since(oldest);
        if age.num_days() > i64::try_from(max_age_days).unwrap_or(i64::MAX) {
            bail!(
                "SslicTcl source data is {} days old (oldest retrieval {oldest}); refresh it with make update-source-data, or set SSLICTCL_SOURCE_DATA_WAIVER with a documented reason",
                age.num_days()
            );
        }
    }

    let files = object
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("provenance manifest needs a files array"))?;
    if files.is_empty() {
        bail!("provenance manifest files array is empty");
    }
    let mut expected = BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        let file = file
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("files[{index}] must be an object"))?;
        let path = file.get("path").and_then(Value::as_str).unwrap_or("");
        let kind = file.get("kind").and_then(Value::as_str).unwrap_or("");
        let digest = file.get("sha256").and_then(Value::as_str).unwrap_or("");
        if !valid_relative_path(path) {
            bail!("files[{index}].path is not a safe relative path: {path:?}");
        }
        if kind != "raw" && kind != "generated" {
            bail!("files[{index}].kind must be raw or generated");
        }
        if !valid_digest(digest) {
            bail!("files[{index}].sha256 must be 64 lowercase hexadecimal characters");
        }
        let expected_root = match kind {
            "raw" => RAW_DIR,
            "generated" => GENERATED_DIR,
            _ => unreachable!(),
        };
        if !path.starts_with(&format!("{expected_root}/")) {
            bail!("files[{index}] path {path:?} does not belong to its {kind} tree");
        }
        if expected
            .insert(path.to_owned(), digest.to_owned())
            .is_some()
        {
            bail!("provenance manifest lists {path:?} more than once");
        }
    }

    let actual = data_files(root, &root.join(RAW_DIR))
        .into_iter()
        .chain(data_files(root, &root.join(GENERATED_DIR)))
        .collect::<BTreeSet<_>>();
    let listed = expected.keys().cloned().collect::<BTreeSet<_>>();
    if actual != listed {
        let missing = listed.difference(&actual).cloned().collect::<Vec<_>>();
        let unlisted = actual.difference(&listed).cloned().collect::<Vec<_>>();
        bail!("provenance file inventory differs (missing: {missing:?}; unlisted: {unlisted:?})");
    }
    for (path, wanted) in expected {
        let bytes = fs::read(root.join(&path)).with_context(|| format!("reading {path}"))?;
        let actual = hex_digest(&bytes);
        if actual != wanted {
            bail!("sha256 mismatch for {path}: manifest {wanted}, actual {actual}");
        }
    }
    Ok(())
}

fn validate_schema(object: &serde_json::Map<String, Value>) -> Result<()> {
    let schema = object
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("provenance manifest needs numeric schema_version"))?;
    if schema != 1 {
        bail!("unsupported SslicTcl provenance schema_version {schema}; expected 1");
    }
    Ok(())
}

fn data_files(root: &Path, directory: &Path) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
    visit_files(root, directory, &mut files);
    files
}

fn visit_files(root: &Path, directory: &Path, files: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_files(root, &path, files);
        } else if path.is_file() {
            files.insert(display_relative(root, &path));
        }
    }
}

fn find_hook(root: &Path, candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|path| root.join(path))
        .find(|path| path.is_file())
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| path.display().to_string(),
        |path| path.display().to_string(),
    )
}

fn valid_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}

fn valid_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_paths_that_escape_data_tree() {
        assert!(!valid_relative_path("../outside"));
        assert!(!valid_relative_path("/absolute"));
        assert!(valid_relative_path("rust/tcl-sslictcl/data/raw/root.pem"));
    }

    #[test]
    fn accepts_lowercase_sha256_only() {
        assert!(valid_digest(&"a".repeat(64)));
        assert!(!valid_digest(&"A".repeat(64)));
        assert!(!valid_digest("short"));
    }

    #[test]
    fn validates_a_complete_manifest_and_hash_inventory() {
        let root =
            std::env::temp_dir().join(format!("tcl-lsp-sslictcl-data-test-{}", std::process::id()));
        let raw = root.join(RAW_DIR);
        let generated = root.join(GENERATED_DIR);
        fs::create_dir_all(&raw).expect("raw directory");
        fs::create_dir_all(&generated).expect("generated directory");
        let raw_path = raw.join("input.txt");
        let generated_path = generated.join("output.json");
        fs::write(&raw_path, b"source").expect("raw file");
        fs::write(&generated_path, b"generated").expect("generated file");
        let raw_relative = display_relative(&root, &raw_path);
        let generated_relative = display_relative(&root, &generated_path);
        let manifest = json!({
            "schema_version": 1,
            "sources": [{
                "id": "test",
                "url": "https://example.invalid/source",
                "revision": "test-revision",
                "retrieved_at": "2026-08-16T00:00:00Z",
                "license": "MIT"
            }],
            "files": [
                {"path": raw_relative, "kind": "raw", "sha256": hex_digest(b"source")},
                {"path": generated_relative, "kind": "generated", "sha256": hex_digest(b"generated")}
            ]
        });
        validate_manifest(&root, &manifest, None).expect("valid manifest");
        fs::write(&generated_path, b"tampered").expect("tamper generated file");
        assert!(validate_manifest(&root, &manifest, None).is_err());
        fs::remove_dir_all(&root).expect("remove test tree");
    }

    #[test]
    fn source_data_scripts_keep_macos_compatible_fallbacks() {
        let root = repo_root();
        let generator =
            fs::read_to_string(root.join("rust/tcl-sslictcl/scripts/generate-source-data.sh"))
                .expect("read generator script");
        assert!(generator.contains("shasum -a 256"));
        assert!(generator.contains("base64 < \"$1\" | tr -d '\\n'"));
        assert!(generator.contains("date -u -j -f"));

        let updater =
            fs::read_to_string(root.join("rust/tcl-sslictcl/scripts/update-source-data.sh"))
                .expect("read updater script");
        assert!(updater.contains("shasum -a 256"));
        assert!(updater.contains("base64 -D"));
    }
}
