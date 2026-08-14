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

//! Reading a release archive in the browser.
//!
//! The multi-version importer wants one set of Tcl sources per release, and the
//! way a user actually has those to hand is a `.zip` — GitHub's "Download ZIP",
//! a release asset, a vendor tarball someone re-zipped. This module turns those
//! bytes into the `{name, text}` list [`crate::import_package_versions`] takes.
//!
//! Nothing here touches a filesystem, so the zip-slip half of
//! `tcl-pkg`'s `extract_zip` hardening has no analogue: a member called
//! `../../etc/passwd` is just a string that ends up in a JSON field. What *does*
//! carry over is the resource half — an entry count cap, a per-entry size cap,
//! and a total-uncompressed cap — because a zip bomb aimed at a browser tab is
//! the same attack it is anywhere else, and the tab has no OOM killer to save
//! it. Every cap is reported rather than silently applied: the caller is told
//! what was skipped and why.

use std::io::{Cursor, Read};

use serde_json::{Value, json};

/// The extensions a Tcl library is written in — the same set the studio's
/// directory import keeps (`studio.ts`'s `TCL_EXTENSIONS`).
const TCL_EXTENSIONS: [&str; 5] = [".tcl", ".tm", ".test", ".itcl", ".itk"];

/// Total uncompressed bytes across every extracted entry.
///
/// Two orders of magnitude below `tcl-pkg`'s 256 MiB, deliberately: that limit
/// guards a disk, this one guards a tab's heap, and a Tcl package's *sources*
/// are measured in megabytes even for the largest libraries.
const MAX_TOTAL_BYTES: u64 = 32 * 1024 * 1024;

/// Cap on a single member's uncompressed size.
const MAX_ENTRY_BYTES: u64 = 8 * 1024 * 1024;

/// Cap on how many members the archive may declare at all.
const MAX_ENTRIES: usize = 20_000;

/// One extracted source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The member's path inside the archive, verbatim.
    pub name: String,
    /// Its contents, decoded as UTF-8 (lossily — a stray Latin-1 byte in a
    /// comment must not lose the whole file).
    pub text: String,
}

/// What reading one archive produced.
#[derive(Debug, Clone, Default)]
pub struct Extraction {
    /// Tcl sources, in archive order.
    pub entries: Vec<Entry>,
    /// Members present but not extracted, and why — the trust surface.
    pub skipped: Vec<String>,
    /// How many members the archive declared, before any filtering.
    pub members: usize,
    /// Total uncompressed bytes of the extracted entries.
    pub bytes: u64,
}

impl Extraction {
    /// The JSON the wasm export returns.
    fn to_json(&self) -> Value {
        json!({
            "entries": self
                .entries
                .iter()
                .map(|e| json!({ "name": e.name, "text": e.text }))
                .collect::<Vec<_>>(),
            "skipped": self.skipped.clone(),
            "members": self.members,
            "bytes": self.bytes,
        })
    }
}

/// Whether `name` looks like a Tcl source we can draft specs from.
fn is_tcl_source(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    TCL_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Read every Tcl source out of the zip in `bytes`.
///
/// Errors only when the container itself cannot be read; a member that cannot
/// be is recorded in [`Extraction::skipped`] and the rest are still returned,
/// because one corrupt file in a release archive should not cost the author the
/// other two hundred.
pub fn extract(bytes: &[u8]) -> Result<Extraction, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("cannot read the archive: {e}"))?;
    let mut out = Extraction {
        members: archive.len(),
        ..Extraction::default()
    };
    if out.members > MAX_ENTRIES {
        return Err(format!(
            "archive declares {} members, over the {MAX_ENTRIES} limit",
            out.members
        ));
    }
    for index in 0..archive.len() {
        let Ok(member) = archive.by_index(index) else {
            out.skipped.push(format!("entry {index}: corrupt header"));
            continue;
        };
        if member.is_dir() {
            continue;
        }
        let name = member.name().to_owned();
        if !is_tcl_source(&name) {
            continue;
        }
        let size = member.size();
        if size > MAX_ENTRY_BYTES {
            out.skipped
                .push(format!("{name}: {size} bytes, over the per-file limit"));
            continue;
        }
        if out.bytes.saturating_add(size) > MAX_TOTAL_BYTES {
            out.skipped.push(format!(
                "{name}: skipped — total extracted size limit reached"
            ));
            continue;
        }
        let mut raw = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
        // `take` bounds the read by the *declared* size, so a member whose
        // header lies about how small it is cannot run the heap away.
        if let Err(e) = member.take(MAX_ENTRY_BYTES).read_to_end(&mut raw) {
            out.skipped.push(format!("{name}: could not be read ({e})"));
            continue;
        }
        out.bytes = out.bytes.saturating_add(raw.len() as u64);
        out.entries.push(Entry {
            name,
            text: String::from_utf8_lossy(&raw).into_owned(),
        });
    }
    Ok(out)
}

/// [`extract`]'s reply as the JSON the export returns, errors included.
pub fn extract_json(bytes: &[u8]) -> Value {
    match extract(bytes) {
        Ok(extraction) => extraction.to_json(),
        Err(message) => json!({ "error": message }),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{Entry, extract, extract_json, is_tcl_source};

    /// Build a zip in memory from `(name, contents)` pairs.
    fn zip_of(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, body) in members {
            writer.start_file(*name, options).expect("start entry");
            writer.write_all(body).expect("write entry");
        }
        writer.finish().expect("finish zip").into_inner()
    }

    #[test]
    fn keeps_tcl_sources_and_drops_everything_else() {
        let bytes = zip_of(&[
            ("mylib-1.0/lib.tcl", b"proc greet {who} {}\n"),
            ("mylib-1.0/README.md", b"# docs\n"),
            ("mylib-1.0/logo.png", b"\x89PNG"),
            ("mylib-1.0/suite.test", b"test a-1 {} {}\n"),
        ]);
        let out = extract(&bytes).expect("readable archive");
        assert_eq!(out.members, 4);
        assert_eq!(
            out.entries,
            vec![
                Entry {
                    name: "mylib-1.0/lib.tcl".to_owned(),
                    text: "proc greet {who} {}\n".to_owned(),
                },
                Entry {
                    name: "mylib-1.0/suite.test".to_owned(),
                    text: "test a-1 {} {}\n".to_owned(),
                },
            ]
        );
        assert!(out.skipped.is_empty());
        assert_eq!(out.bytes, 20 + 15);
    }

    #[test]
    fn recognises_every_tcl_extension_case_insensitively() {
        for name in ["a.tcl", "b.TM", "c.Test", "d.itcl", "e.ITK"] {
            assert!(is_tcl_source(name), "{name} should be a Tcl source");
        }
        for name in ["a.tcl.bak", "notes.txt", "tcl", "x.tclspec"] {
            assert!(!is_tcl_source(name), "{name} should not be a Tcl source");
        }
    }

    #[test]
    fn decodes_non_utf8_bytes_lossily_rather_than_losing_the_file() {
        let bytes = zip_of(&[("p.tcl", b"# \xff caf\xe9\nproc p {} {}\n")]);
        let out = extract(&bytes).expect("readable archive");
        assert_eq!(out.entries.len(), 1);
        assert!(out.entries[0].text.contains("proc p {} {}"));
    }

    #[test]
    fn a_directory_member_is_not_an_entry() {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        writer.add_directory("pkg/", options).expect("dir");
        writer.start_file("pkg/a.tcl", options).expect("file");
        writer.write_all(b"proc a {} {}").expect("write");
        let bytes = writer.finish().expect("finish").into_inner();
        let out = extract(&bytes).expect("readable archive");
        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0].name, "pkg/a.tcl");
    }

    #[test]
    fn a_member_over_the_per_file_cap_is_skipped_and_reported() {
        let big = vec![b' '; 9 * 1024 * 1024];
        let bytes = zip_of(&[("huge.tcl", &big), ("small.tcl", b"proc s {} {}")]);
        let out = extract(&bytes).expect("readable archive");
        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0].name, "small.tcl");
        assert_eq!(out.skipped.len(), 1);
        assert!(out.skipped[0].starts_with("huge.tcl: "));
        assert!(out.skipped[0].contains("per-file limit"));
    }

    #[test]
    fn bytes_that_are_not_a_zip_are_an_error_object_not_a_panic() {
        let value = extract_json(b"this is not a zip archive at all");
        assert!(
            value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|e| e.contains("cannot read the archive")),
            "unexpected reply: {value}"
        );
    }

    #[test]
    fn json_carries_entries_skips_and_counters() {
        let bytes = zip_of(&[("a.tcl", b"proc a {} {}"), ("b.txt", b"hi")]);
        let value = extract_json(&bytes);
        assert_eq!(value["members"], 2);
        assert_eq!(value["entries"].as_array().expect("array").len(), 1);
        assert_eq!(value["entries"][0]["name"], "a.tcl");
        assert_eq!(value["entries"][0]["text"], "proc a {} {}");
        assert_eq!(value["skipped"].as_array().expect("array").len(), 0);
        assert_eq!(value["bytes"], 12);
    }
}
