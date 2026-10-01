// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline provenance, version, hash, and embedded-file inventory gate for the
//! Tcl standard-library read-closure shipped by `runtime/rust`.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tcl_dialect::TclVersion;

use crate::util::repo_root;

const VENDOR_DIR: &str = "runtime/rust/vendor/tcl_library";
const MANIFEST: &str = "runtime/rust/vendor/tcl_library/manifest.json";
const EMBED_OWNER: &str = "runtime/rust/src/embedded_stdlib.rs";

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    tcl_patchlevel: String,
    source_url: String,
    source_revision: String,
    files: Vec<FileEntry>,
}

#[derive(Deserialize)]
struct FileEntry {
    path: String,
    source: String,
    sha256: String,
    embedded: bool,
}

pub fn run() -> Result<ExitCode> {
    let root = repo_root();
    let text =
        fs::read_to_string(root.join(MANIFEST)).context("reading runtime stdlib manifest")?;
    let manifest: Manifest =
        serde_json::from_str(&text).context("parsing runtime stdlib manifest")?;
    if manifest.schema_version != 1 {
        bail!(
            "unsupported runtime stdlib manifest schema {}",
            manifest.schema_version
        );
    }
    let expected_patch = TclVersion::V9_0.patchlevel();
    if manifest.tcl_patchlevel != expected_patch {
        bail!(
            "runtime stdlib is Tcl {}, expected {expected_patch}",
            manifest.tcl_patchlevel
        );
    }
    if manifest.source_url.is_empty() || manifest.source_revision.len() != 40 {
        bail!("runtime stdlib manifest has incomplete source provenance");
    }

    let mut listed = BTreeSet::new();
    let mut embedded = BTreeSet::new();
    for entry in &manifest.files {
        if entry.path.starts_with('/')
            || entry.path.contains("..")
            || !listed.insert(entry.path.clone())
        {
            bail!("invalid or duplicate runtime stdlib path {:?}", entry.path);
        }
        if entry.source.is_empty() {
            bail!(
                "runtime stdlib path {:?} has no upstream source",
                entry.path
            );
        }
        let bytes = fs::read(root.join(VENDOR_DIR).join(&entry.path))
            .with_context(|| format!("reading runtime stdlib file {}", entry.path))?;
        let mut actual = String::with_capacity(64);
        for byte in Sha256::digest(&bytes) {
            write!(&mut actual, "{byte:02x}").expect("writing a digest to a String cannot fail");
        }
        if actual != entry.sha256 {
            bail!(
                "runtime stdlib hash drift for {}: manifest {}, actual {actual}",
                entry.path,
                entry.sha256
            );
        }
        if entry.embedded {
            embedded.insert(entry.path.clone());
        }
    }

    let actual_files = vendored_files(&root.join(VENDOR_DIR))?;
    if listed != actual_files {
        bail!("runtime stdlib manifest file inventory drift");
    }
    let owner =
        fs::read_to_string(root.join(EMBED_OWNER)).context("reading embedded stdlib owner")?;
    let owner_files = embedded_owner_files(&owner)?;
    if embedded != owner_files {
        bail!("embedded stdlib FILES table disagrees with manifest read-closure");
    }
    let revision = embedded_revision(&manifest);
    if revision != tcl_runtime_api::manifest::EMBEDDED_STDLIB_REVISION {
        bail!(
            "EMBEDDED_STDLIB_REVISION in rust/tcl-runtime-api/src/manifest.rs is {:?}, but the \
             vendored library's manifest says {revision:?}: every artefact states the revision \
             of the library it assumed, so update the constant with the library",
            tcl_runtime_api::manifest::EMBEDDED_STDLIB_REVISION
        );
    }
    let init = fs::read_to_string(root.join(VENDOR_DIR).join("init.tcl"))?;
    let exact = format!("package require -exact tcl {expected_patch}");
    if !init.lines().any(|line| line.trim() == exact) {
        bail!("vendored init.tcl does not require exact Tcl {expected_patch}");
    }
    let readme = fs::read_to_string(root.join(VENDOR_DIR).join("README.md"))?;
    let named = named_patchlevels(&readme);
    if named.len() != 1 || !named.contains(expected_patch) {
        bail!("runtime stdlib README does not identify only Tcl {expected_patch}");
    }
    eprintln!(
        "runtime stdlib: Tcl {} · {} embedded files · provenance {}",
        manifest.tcl_patchlevel,
        embedded.len(),
        manifest.source_revision
    );
    Ok(ExitCode::SUCCESS)
}

/// The revision the runtimes state of the library they embed: the patchlevel,
/// the first twelve characters of the upstream commit, and the first twelve of
/// the SHA-256 over the embedded files' paths and hashes, so a refreshed file
/// moves it whichever way the refresh was made.
fn embedded_revision(manifest: &Manifest) -> String {
    let mut embedded: Vec<&FileEntry> = manifest.files.iter().filter(|e| e.embedded).collect();
    embedded.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hasher = Sha256::new();
    for entry in embedded {
        hasher.update(format!("{}\t{}\n", entry.path, entry.sha256));
    }
    let mut digest = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(&mut digest, "{byte:02x}").expect("writing a digest to a String cannot fail");
    }
    format!(
        "{}+{}.{}",
        manifest.tcl_patchlevel,
        &manifest.source_revision[..12],
        &digest[..12]
    )
}

/// Every three-component Tcl patchlevel the text names, so a patchlevel left
/// behind by an earlier vendor refresh fails the gate as loudly as a missing one.
fn named_patchlevels(text: &str) -> BTreeSet<&str> {
    text.match_indices("Tcl ")
        .filter_map(|(index, marker)| {
            let rest = &text[index + marker.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .unwrap_or(rest.len());
            let token = rest[..end].trim_end_matches('.');
            (token.matches('.').count() == 2).then_some(token)
        })
        .collect()
}

fn vendored_files(directory: &std::path::Path) -> Result<BTreeSet<String>> {
    let mut files = BTreeSet::new();
    collect_files(directory, directory, &mut files)?;
    files.remove("README.md");
    files.remove("manifest.json");
    Ok(files)
}

fn collect_files(
    root: &std::path::Path,
    directory: &std::path::Path,
    files: &mut BTreeSet<String>,
) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            files.insert(path.strip_prefix(root)?.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

fn embedded_owner_files(source: &str) -> Result<BTreeSet<String>> {
    let body = source
        .split_once("static FILES")
        .and_then(|(_, tail)| tail.split_once("];").map(|(body, _)| body))
        .ok_or_else(|| anyhow::anyhow!("cannot locate embedded stdlib FILES table"))?;
    Ok(body
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split_once('"').map(|(path, _)| path.to_owned()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, sha256: &str, embedded: bool) -> FileEntry {
        FileEntry {
            path: path.to_owned(),
            source: path.to_owned(),
            sha256: sha256.to_owned(),
            embedded,
        }
    }

    fn manifest(files: Vec<FileEntry>) -> Manifest {
        Manifest {
            schema_version: 1,
            tcl_patchlevel: "9.0.4".to_owned(),
            source_url: "https://example.invalid/tcl".to_owned(),
            source_revision: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            files,
        }
    }

    #[test]
    fn the_revision_moves_with_every_embedded_file_and_no_other() {
        let base = embedded_revision(&manifest(vec![
            entry("b.tcl", "bb", true),
            entry("a.tcl", "aa", true),
            entry("license.terms", "ll", false),
        ]));
        assert!(base.starts_with("9.0.4+0123456789ab."), "{base}");
        assert_eq!(base.len(), "9.0.4+0123456789ab.".len() + 12);

        // Listing order, and what is not embedded, do not matter.
        assert_eq!(
            base,
            embedded_revision(&manifest(vec![
                entry("license.terms", "changed", false),
                entry("a.tcl", "aa", true),
                entry("b.tcl", "bb", true),
            ]))
        );
        // A file that changes, goes or arrives does.
        for changed in [
            vec![entry("b.tcl", "bX", true), entry("a.tcl", "aa", true)],
            vec![entry("a.tcl", "aa", true)],
            vec![
                entry("b.tcl", "bb", true),
                entry("a.tcl", "aa", true),
                entry("c.tcl", "cc", true),
            ],
        ] {
            assert_ne!(base, embedded_revision(&manifest(changed)));
        }
    }

    #[test]
    fn parses_only_the_files_table_literals() {
        let source = "static FILES: X = embed![\n  \"init.tcl\",\n  \"x/pkgIndex.tcl\",\n];";
        assert_eq!(
            embedded_owner_files(source).expect("table"),
            BTreeSet::from(["init.tcl".to_owned(), "x/pkgIndex.tcl".to_owned()])
        );
    }
}
