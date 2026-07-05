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

//! `tclpkg.lock` lockfile — canonical JSON representation.
//!
//! The lockfile records the
//! exact resolved dependency graph. Two invocations against the same manifest
//! and registry snapshot produce byte-identical output (only the `generated`
//! timestamp varies, and `--frozen` preserves it). Canonical JSON rules:
//! 2-space indent, alphabetically sorted keys, LF endings, final newline,
//! `packages[]` sorted by `(name, version)`.

use std::path::Path;

use chrono::Utc;
use serde_json::{Value, json};

use crate::errors::TclPkgError;
use crate::json::canonical_json;

/// Schema version — bumped only on incompatible lockfile changes.
pub const LOCKFILE_VERSION: i64 = 1;

/// Generator string embedded in the lockfile.
const GENERATOR: &str = "tcl pkg";

/// Where a package was fetched from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpec {
    pub kind: String, // "tarball" | "git" | "path"
    pub url: String,
    pub subdir: String,
    pub rev: Option<String>,
}

impl SourceSpec {
    #[must_use]
    pub fn new(kind: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            url: url.into(),
            subdir: String::new(),
            rev: None,
        }
    }

    fn to_value(&self) -> Value {
        json!({
            "type": self.kind,
            "url": self.url,
            "subdir": self.subdir,
            "rev": self.rev,
        })
    }

    fn from_value(v: &Value) -> Self {
        Self {
            kind: str_field(v, "type", ""),
            url: str_field(v, "url", ""),
            subdir: str_field(v, "subdir", ""),
            rev: v
                .get("rev")
                .and_then(|r| r.as_str().map(ToString::to_string)),
        }
    }
}

/// A single resolved package in the lockfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: SourceSpec,
    pub integrity: String,
    pub size: i64,
    pub requires: Vec<String>,
    pub provides: Vec<String>,
    pub license: String,
    pub dev: bool,
}

impl LockedPackage {
    fn to_value(&self) -> Value {
        let mut provides = self.provides.clone();
        provides.sort();
        let mut requires = self.requires.clone();
        requires.sort();
        json!({
            "dev": self.dev,
            "integrity": self.integrity,
            "license": self.license,
            "name": self.name,
            "provides": provides,
            "requires": requires,
            "size": self.size,
            "source": self.source.to_value(),
            "version": self.version,
        })
    }

    fn from_value(v: &Value) -> Self {
        Self {
            name: str_field(v, "name", ""),
            version: str_field(v, "version", ""),
            source: SourceSpec::from_value(v.get("source").unwrap_or(&Value::Null)),
            integrity: str_field(v, "integrity", ""),
            size: v.get("size").and_then(Value::as_i64).unwrap_or(0),
            requires: str_list(v, "requires"),
            provides: str_list(v, "provides"),
            license: str_field(v, "license", ""),
            dev: v.get("dev").and_then(Value::as_bool).unwrap_or(false),
        }
    }
}

/// A `replace` directive that was applied during resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceEntry {
    pub name: String,
    pub from_version: String,
    pub to_version: String,
}

impl ReplaceEntry {
    fn to_value(&self) -> Value {
        json!({"from": self.from_version, "name": self.name, "to": self.to_version})
    }

    fn from_value(v: &Value) -> Self {
        Self {
            name: str_field(v, "name", ""),
            from_version: str_field(v, "from", ""),
            to_version: str_field(v, "to", ""),
        }
    }
}

/// An `exclude` directive that was honoured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludeEntry {
    pub name: String,
    pub version: String,
}

impl ExcludeEntry {
    fn to_value(&self) -> Value {
        json!({"name": self.name, "version": self.version})
    }

    fn from_value(v: &Value) -> Self {
        Self {
            name: str_field(v, "name", ""),
            version: str_field(v, "version", ""),
        }
    }
}

/// In-memory representation of a `tclpkg.lock` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockFile {
    pub version: i64,
    pub name: String,
    pub tcl: String,
    pub generated: String,
    pub generator: String,
    pub root_hash: String,
    pub packages: Vec<LockedPackage>,
    pub replaces: Vec<ReplaceEntry>,
    pub excludes: Vec<ExcludeEntry>,
}

impl Default for LockFile {
    fn default() -> Self {
        Self {
            version: LOCKFILE_VERSION,
            name: String::new(),
            tcl: ">=8.6".to_string(),
            generated: String::new(),
            generator: GENERATOR.to_string(),
            root_hash: String::new(),
            packages: Vec::new(),
            replaces: Vec::new(),
            excludes: Vec::new(),
        }
    }
}

impl LockFile {
    /// A fresh lockfile for `name` with the given Tcl constraint.
    #[must_use]
    pub fn new(name: impl Into<String>, tcl: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tcl: tcl.into(),
            ..Self::default()
        }
    }

    fn to_value(&self) -> Value {
        let mut excludes: Vec<&ExcludeEntry> = self.excludes.iter().collect();
        excludes.sort_by(|a, b| a.name.cmp(&b.name));
        let mut packages: Vec<&LockedPackage> = self.packages.iter().collect();
        packages.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
        let mut replaces: Vec<&ReplaceEntry> = self.replaces.iter().collect();
        replaces.sort_by(|a, b| a.name.cmp(&b.name));
        json!({
            "excludes": excludes.iter().map(|e| e.to_value()).collect::<Vec<_>>(),
            "generated": self.generated,
            "generator": self.generator,
            "name": self.name,
            "packages": packages.iter().map(|p| p.to_value()).collect::<Vec<_>>(),
            "replaces": replaces.iter().map(|r| r.to_value()).collect::<Vec<_>>(),
            "root_hash": self.root_hash,
            "tcl": self.tcl,
            "version": self.version,
        })
    }

    fn from_value(v: &Value) -> Self {
        let mut lf = LockFile {
            version: v
                .get("version")
                .and_then(Value::as_i64)
                .unwrap_or(LOCKFILE_VERSION),
            name: str_field(v, "name", ""),
            tcl: str_field(v, "tcl", ">=8.6"),
            generated: str_field(v, "generated", ""),
            generator: str_field(v, "generator", GENERATOR),
            root_hash: str_field(v, "root_hash", ""),
            ..LockFile::default()
        };
        if let Some(arr) = v.get("packages").and_then(Value::as_array) {
            lf.packages = arr.iter().map(LockedPackage::from_value).collect();
        }
        if let Some(arr) = v.get("replaces").and_then(Value::as_array) {
            lf.replaces = arr.iter().map(ReplaceEntry::from_value).collect();
        }
        if let Some(arr) = v.get("excludes").and_then(Value::as_array) {
            lf.excludes = arr.iter().map(ExcludeEntry::from_value).collect();
        }
        lf
    }

    /// Return the locked entry for `name`, if any.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Set `generated` to now (UTC, ISO-8601, second precision, `Z` suffix).
    pub fn stamp(&mut self) {
        self.generated = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    }
}

fn str_field(v: &Value, key: &str, default: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

fn str_list(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Produce canonical JSON for a lockfile, including the final newline.
#[must_use]
pub fn serialise(lockfile: &LockFile) -> String {
    let mut text = canonical_json(&lockfile.to_value());
    text.push('\n');
    text
}

/// Parse a lockfile JSON string.
pub fn deserialise(text: &str) -> Result<LockFile, TclPkgError> {
    let data: Value = serde_json::from_str(text)
        .map_err(|exc| TclPkgError::new(format!("invalid lockfile JSON: {exc}")))?;
    if !data.is_object() {
        return Err(TclPkgError::new("lockfile must be a JSON object"));
    }
    if let Some(schema_version) = data.get("version").and_then(Value::as_i64)
        && schema_version > LOCKFILE_VERSION
    {
        return Err(TclPkgError::new(format!(
            "lockfile schema version {schema_version} is newer than supported \
                 ({LOCKFILE_VERSION}); upgrade tcl-lsp"
        )));
    }
    Ok(LockFile::from_value(&data))
}

/// Write the lockfile to disk atomically (temp file then rename).
pub fn write_lockfile(lockfile: &LockFile, path: impl AsRef<Path>) -> Result<(), TclPkgError> {
    let dest = path.as_ref();
    let text = serialise(lockfile);
    let tmp = dest.with_extension("lock.tmp");
    std::fs::write(&tmp, &text)
        .map_err(|e| TclPkgError::new(format!("cannot write lockfile {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        TclPkgError::new(format!("cannot write lockfile {}: {e}", dest.display()))
    })?;
    Ok(())
}

/// Read a lockfile from disk.
pub fn read_lockfile(path: impl AsRef<Path>) -> Result<LockFile, TclPkgError> {
    let file_path = path.as_ref();
    if !file_path.is_file() {
        return Err(TclPkgError::manifest(
            format!("lockfile not found: {}", file_path.display()),
            None,
            None,
            Some("run 'tcl pkg install' to create one".to_string()),
        ));
    }
    let text = std::fs::read_to_string(file_path)
        .map_err(|e| TclPkgError::new(format!("cannot read lockfile: {e}")))?;
    deserialise(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LockFile {
        let mut lf = LockFile::new("myapp", ">=8.6");
        lf.generated = "2024-01-01T00:00:00Z".to_string();
        lf.packages.push(LockedPackage {
            name: "json".to_string(),
            version: "1.3.5".to_string(),
            source: SourceSpec::new("tarball", "https://example.org/json.tar.gz"),
            integrity: "sha256-abc".to_string(),
            size: 0,
            requires: vec!["http@2.0".to_string()],
            provides: vec![],
            license: "MIT".to_string(),
            dev: false,
        });
        lf
    }

    #[test]
    fn round_trips() {
        let lf = sample();
        let text = serialise(&lf);
        let parsed = deserialise(&text).unwrap();
        assert_eq!(parsed.name, "myapp");
        assert_eq!(parsed.packages.len(), 1);
        assert_eq!(parsed.packages[0].version, "1.3.5");
        // Re-serialising is byte-stable.
        assert_eq!(serialise(&parsed), text);
    }

    #[test]
    fn canonical_shape() {
        let lf = sample();
        let text = serialise(&lf);
        assert!(text.ends_with('\n'));
        // Keys are alphabetically sorted: excludes < generated < ... < version.
        let excludes_pos = text.find("\"excludes\"").unwrap();
        let generated_pos = text.find("\"generated\"").unwrap();
        let version_pos = text.rfind("\"version\"").unwrap();
        assert!(excludes_pos < generated_pos);
        assert!(generated_pos < version_pos);
        // 2-space indent on the top-level keys.
        assert!(text.contains("\n  \"name\": \"myapp\""));
    }

    #[test]
    fn rejects_newer_schema() {
        let err = deserialise("{\"version\": 999}").unwrap_err();
        assert!(err.to_string().contains("newer than supported"));
    }

    #[test]
    fn lookup_works() {
        let lf = sample();
        assert_eq!(lf.lookup("json").unwrap().version, "1.3.5");
        assert!(lf.lookup("missing").is_none());
    }
}
