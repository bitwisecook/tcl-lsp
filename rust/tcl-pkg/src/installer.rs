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

//! Install orchestration — source resolution, fetch, CAS store, materialise.
//!
//! `tcl pkg install` resolves the dependency graph, then for each resolved
//! package this module turns a
//! `name@version` into a concrete source, fetches it ([`crate::fetchers`]),
//! ingests it into the content-addressable store ([`crate::cas`]), and
//! materialises it into the project `lib/`. The lockfile records the exact
//! source, integrity hash, size, and `provides`/`license` read back from the
//! fetched package's own manifest.
//!
//! Source resolution order: a root `replace` directive, then a
//! `require`/`dev-require` `-source URL`, then the registry's first source
//! entry. When no source is determinable — or a fetch fails / the registry is
//! unreachable — the caller keeps a placeholder lock entry and warns rather
//! than aborting.

// `.git` / `git+` are URL markers, not filesystem extensions, so
// `Path::extension` would be the wrong tool here.
#![allow(clippy::case_sensitive_file_extension_comparisons)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cas::{ContentAddressableStore, tree_size};
use crate::errors::TclPkgError;
use crate::fetchers::{fetch_git, fetch_path, fetch_tarball};
use crate::lockfile::SourceSpec;
use crate::registry::RegistryClient;

/// Outcome of fetching + storing + reading one package.
#[derive(Debug, Clone)]
pub struct MaterialiseResult {
    pub source: SourceSpec,
    pub integrity: String,
    pub size: i64,
    pub provides: Vec<String>,
    pub license: String,
}

fn classify(url: &str, rev: Option<&str>) -> SourceSpec {
    if Path::new(url).is_dir() {
        return SourceSpec::new("path", url);
    }
    if url.starts_with("git+") || url.ends_with(".git") || url.starts_with("git@") {
        let mut spec = SourceSpec::new("git", url);
        spec.rev = rev.map(ToString::to_string);
        return spec;
    }
    SourceSpec::new("tarball", url)
}

/// Return the [`SourceSpec`] for `name@version`, or `None`.
///
/// `replaces`/`manifest_sources` map package name → source URL. The registry is
/// consulted last and any registry error is swallowed into a `None` result so
/// the caller can degrade gracefully.
#[must_use]
#[allow(clippy::implicit_hasher)] // callers always pass a std `HashMap`
pub fn resolve_source(
    name: &str,
    version: &str,
    replaces: &HashMap<String, String>,
    manifest_sources: &HashMap<String, String>,
    registry: Option<&mut RegistryClient>,
) -> Option<SourceSpec> {
    if let Some(url) = replaces.get(name) {
        return Some(classify(url, Some(version)));
    }
    if let Some(url) = manifest_sources.get(name) {
        return Some(classify(url, Some(version)));
    }
    if let Some(registry) = registry {
        let entry = registry.lookup(name).ok().flatten()?;
        if let Some(src) = entry.sources.first() {
            if src.method == "git" || src.url.ends_with(".git") {
                let mut spec = SourceSpec::new("git", &src.url);
                spec.rev = Some(version.to_string());
                return Some(spec);
            }
            return Some(SourceSpec::new("tarball", &src.url));
        }
    }
    None
}

fn read_pkg_meta(root: &Path) -> (Vec<String>, String) {
    let manifest_path = root.join("tclpkg.tcl");
    if !manifest_path.is_file() {
        return (Vec::new(), String::new());
    }
    match crate::manifest::load_manifest(&manifest_path) {
        Ok(ast) => (ast.provides, ast.license),
        Err(_) => (Vec::new(), String::new()),
    }
}

/// A temporary directory removed on drop (no external `tempfile` dependency).
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, TclPkgError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("tclpkg-fetch-{}-{nanos}-{seq}", std::process::id()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| TclPkgError::new(format!("cannot create temp dir: {e}")))?;
        Ok(TempDir(dir))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Fetch `source` into a temp dir, ingest it into the CAS, read metadata.
///
/// Returns a [`MaterialiseResult`] with the (possibly rev-resolved) source,
/// integrity hash, size, and package metadata. Errors on fetch/store failure.
pub fn fetch_and_store(
    source: &SourceSpec,
    name: &str,
    version: &str,
    cas: &ContentAddressableStore,
    timeout: u64,
) -> Result<MaterialiseResult, TclPkgError> {
    let tmp = TempDir::new()?;
    let worktree = tmp.path().join("src");

    let resolved_source = match source.kind.as_str() {
        "path" => {
            fetch_path(&source.url, &worktree)?;
            source.clone()
        }
        "git" => {
            let sha = fetch_git(&source.url, &worktree, source.rev.as_deref())?;
            SourceSpec {
                kind: "git".to_string(),
                url: source.url.clone(),
                subdir: source.subdir.clone(),
                rev: if sha.is_empty() {
                    source.rev.clone()
                } else {
                    Some(sha)
                },
            }
        }
        _ => {
            fetch_tarball(&source.url, &worktree, timeout)?;
            source.clone()
        }
    };

    let integrity = cas.store(&worktree, name, version, None)?;
    let size = i64::try_from(tree_size(&worktree)).unwrap_or(i64::MAX);
    let (provides, license) = read_pkg_meta(&worktree);

    Ok(MaterialiseResult {
        source: resolved_source,
        integrity,
        size,
        provides,
        license,
    })
}

/// Materialise a CAS entry into `lib_dir/<name>-<version>`.
pub fn materialise(
    cas: &ContentAddressableStore,
    integrity: &str,
    lib_dir: &Path,
    name: &str,
    version: &str,
    symlink: bool,
) -> Result<PathBuf, TclPkgError> {
    let dest = lib_dir.join(format!("{name}-{version}"));
    cas.materialise(integrity, &dest, symlink)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "tclpkg-installer-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn classify_sources() {
        assert_eq!(classify("https://x/p.tar.gz", None).kind, "tarball");
        assert_eq!(classify("https://x/p.git", Some("1.0")).kind, "git");
        assert_eq!(classify("git+https://x/p", None).kind, "git");
        let dir = tmp("classify");
        assert_eq!(classify(dir.to_str().unwrap(), None).kind, "path");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_prefers_replace_then_source() {
        let mut replaces = HashMap::new();
        replaces.insert("a".to_string(), "https://r/a.tar.gz".to_string());
        let mut sources = HashMap::new();
        sources.insert("a".to_string(), "https://s/a.tar.gz".to_string());
        sources.insert("b".to_string(), "https://s/b.git".to_string());
        // replace wins for `a`
        let sa = resolve_source("a", "1.0", &replaces, &sources, None).unwrap();
        assert_eq!(sa.url, "https://r/a.tar.gz");
        // manifest -source for `b`
        let sb = resolve_source("b", "2.0", &replaces, &sources, None).unwrap();
        assert_eq!(sb.kind, "git");
        // unknown with no registry → None
        assert!(resolve_source("c", "1.0", &replaces, &sources, None).is_none());
    }

    #[test]
    fn fetch_store_materialise_roundtrip() {
        let base = tmp("roundtrip");
        // a local "package" to install
        let pkg = base.join("pkgsrc");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("tclpkg.tcl"),
            "package foo\nversion 1.0.0\nprovides foo::api\nlicense MIT\n",
        )
        .unwrap();
        std::fs::write(pkg.join("foo.tcl"), "proc foo::hello {} { return hi }\n").unwrap();

        let cas = ContentAddressableStore::new(&base.join("cache"));
        let source = SourceSpec::new("path", pkg.to_str().unwrap());
        let result = fetch_and_store(&source, "foo", "1.0.0", &cas, 60).unwrap();
        assert!(result.integrity.starts_with("sha256-"));
        assert!(result.size > 0);
        assert_eq!(result.provides, vec!["foo::api"]);
        assert_eq!(result.license, "MIT");

        let lib = base.join("lib");
        let dest = materialise(&cas, &result.integrity, &lib, "foo", "1.0.0", false).unwrap();
        assert!(dest.is_dir());
        assert_eq!(
            std::fs::read_to_string(dest.join("foo.tcl")).unwrap(),
            "proc foo::hello {} { return hi }\n"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
