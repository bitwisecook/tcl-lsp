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

//! Release snapshots → a `.tclspec` pack whose lifecycle fields carry real
//! version ranges.
//!
//! The shared half of `tcl spec import` (CLI) and `spec_import` (MCP): read a
//! release's sources off disk — a directory, a `.zip`, or a `.tar.gz` — hand
//! the labelled set to [`tcl_spec_studio::versions::import_package_versions`],
//! and render the merged drafts as one pack with the derivation evidence
//! preserved in its comment header.
//!
//! **Nothing here touches the network.** That is the point of the split: the
//! MCP server links this module and gains local-artefact import without
//! gaining a fetcher, while the CLI's `--github` mode keeps its tag
//! enumeration and its downloads in `tcl-cli` where a user asked for them.
//! Archive extraction is not reimplemented either — it is
//! [`tcl_pkg::fetchers::extract_archive`], so a snapshot archive gets the same
//! zip-slip rejection, symlink skipping and decompression-bomb cap a fetched
//! package does.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tcl_spec_studio::infer::SourceFile;
use tcl_spec_studio::render_spectcl::render_pack_reporting;
use tcl_spec_studio::versions::{
    VERSION_GATE_NOTE, VersionedImportOptions, VersionedSnapshot, import_package_versions,
};

/// The file extensions a snapshot's sources are read from.
///
/// The studio's importer filter verbatim (`TCL_EXTENSIONS` in
/// `rust/tcl-spec-studio/web/src/studio.ts`), so a directory import and a
/// drag-and-drop import see the same files. `.test` is in the list because a
/// package's test suite frequently defines the helper procs the package's own
/// files call.
pub const TCL_EXTENSIONS: &[&str] = &[".tcl", ".tm", ".test", ".itcl", ".itk"];

/// What one `--snapshot` path is, decided by its extension alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotKind {
    /// An unpacked release tree.
    Directory,
    /// A `.zip` release archive.
    Zip,
    /// A `.tar`, `.tar.gz` or `.tgz` release archive.
    Tarball,
}

/// Why an import could not be run.
#[derive(Debug, thiserror::Error)]
pub enum SpecImportError {
    /// A `--snapshot` argument that is not `VERSION=PATH`.
    #[error("`{0}` is not VERSION=PATH (for example `--snapshot 1.2=./releases/1.2`)")]
    SnapshotSyntax(String),
    /// No snapshot was given at all: there is no history to diff.
    #[error("no snapshots given; pass at least one --snapshot VERSION=PATH")]
    NoSnapshots,
    /// A snapshot path that is neither an existing directory nor a readable
    /// archive.
    #[error("snapshot {version}: `{path}` is not a directory, .zip or .tar.gz")]
    SnapshotPath {
        /// The release label the path was given under.
        version: String,
        /// The path as the caller spelled it.
        path: String,
    },
    /// An archive that could not be read or extracted.
    #[error("snapshot {version}: {message}")]
    Archive {
        /// The release label the archive was given under.
        version: String,
        /// What the reader or the extractor said.
        message: String,
    },
    /// A snapshot with no Tcl in it — almost always the wrong directory.
    #[error("snapshot {version}: no Tcl sources ({extensions}) found under `{path}`")]
    NoSources {
        /// The release label that came up empty.
        version: String,
        /// Where we looked.
        path: String,
        /// The extension list we looked for, for the message.
        extensions: String,
    },
}

/// What the caller knows about the snapshots, beyond the sources themselves.
#[derive(Debug, Clone, Copy)]
pub struct SpecImportOptions<'a> {
    /// Registry dialect every snapshot is analysed as.
    pub dialect: &'static tcl_dialect::DialectProfile,
    /// Pack name override; otherwise the `package provide` name is used.
    pub package: Option<&'a str>,
    /// Whether the snapshots are *every* release of the package.
    ///
    /// Only with a complete history is presence in the earliest snapshot an
    /// introduction; see [`VersionedImportOptions::complete_history`].
    pub complete_history: bool,
}

/// One command's derived lifecycle, lifted out of the merged draft.
#[derive(Debug, Clone)]
pub struct DerivedCommand {
    /// The qualified command name.
    pub name: String,
    /// First release the command exists in, when the releases witness one.
    pub introduced_version: Option<String>,
    /// First release the command no longer exists in (an exclusive bound).
    pub retired_version: Option<String>,
    /// The evidence behind every guess, one line each.
    pub notes: Vec<String>,
}

impl DerivedCommand {
    /// The `version-gate:` notes: facts the draft model has no field for yet.
    #[must_use]
    pub fn gates(&self) -> Vec<&str> {
        self.notes
            .iter()
            .map(String::as_str)
            .filter(|note| note.starts_with(VERSION_GATE_NOTE))
            .collect()
    }
}

/// A rendered pack plus everything the derivation had to say about it.
#[derive(Debug, Clone)]
pub struct SpecImport {
    /// The pack name used in the `speclib` declaration.
    pub package: String,
    /// The releases analysed, oldest first.
    pub versions: Vec<String>,
    /// One entry per command seen in any release, in name order.
    pub commands: Vec<DerivedCommand>,
    /// Ordering problems and every fact the releases contradict each other on.
    pub warnings: Vec<String>,
    /// Fields the render could not carry, as `key: reason`.
    pub losses: Vec<String>,
    /// The `.tclspec` source, evidence header included.
    pub pack: String,
}

impl SpecImport {
    /// The JSON the MCP tool returns and `--json` prints.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "package": self.package,
            "versions": self.versions.clone(),
            "pack": self.pack.clone(),
            "commands": self.commands.iter().map(|c| json!({
                "name": c.name,
                "introduced_version": c.introduced_version,
                "retired_version": c.retired_version,
                "version_gates": c.gates(),
                "notes": c.notes.clone(),
            })).collect::<Vec<_>>(),
            "warnings": self.warnings.clone(),
            "losses": self.losses.clone(),
        })
    }

    /// How many commands the releases pin an `introduced_version` on.
    #[must_use]
    pub fn introduced_count(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| c.introduced_version.is_some())
            .count()
    }

    /// How many commands the releases retire.
    #[must_use]
    pub fn retired_count(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| c.retired_version.is_some())
            .count()
    }
}

/// Split a `--snapshot VERSION=PATH` argument.
///
/// The first `=` separates the two, so a path may contain further `=`
/// characters; a Windows drive letter is unaffected because it uses `:`.
pub fn parse_snapshot_arg(arg: &str) -> Result<(String, PathBuf), SpecImportError> {
    let (version, path) = arg
        .split_once('=')
        .ok_or_else(|| SpecImportError::SnapshotSyntax(arg.to_owned()))?;
    let version = version.trim();
    if version.is_empty() || path.is_empty() {
        return Err(SpecImportError::SnapshotSyntax(arg.to_owned()));
    }
    Ok((version.to_owned(), PathBuf::from(path)))
}

/// The name suffixes that make a snapshot path a zip archive.
const ZIP_SUFFIXES: &[&str] = &[".zip"];

/// The name suffixes that make a snapshot path a tar archive. `.tar.gz` is a
/// double extension, so this is a suffix list rather than a `Path::extension`
/// comparison.
const TAR_SUFFIXES: &[&str] = &[".tar.gz", ".tgz", ".tar"];

/// What `path` is, by extension: an archive we can unpack, or a directory.
///
/// The name decides, not the filesystem — a caller naming a `.zip` that is
/// really a directory gets an archive error rather than a silent directory
/// walk, which is the diagnosis it wants. The comparison is case-insensitive:
/// the name is lowercased first.
#[must_use]
pub fn snapshot_kind(path: &Path) -> SnapshotKind {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ends_with_any = |suffixes: &[&str]| suffixes.iter().any(|suffix| name.ends_with(suffix));
    if ends_with_any(ZIP_SUFFIXES) {
        SnapshotKind::Zip
    } else if ends_with_any(TAR_SUFFIXES) {
        SnapshotKind::Tarball
    } else {
        SnapshotKind::Directory
    }
}

/// Read one release's sources into a labelled snapshot.
///
/// An archive is unpacked into a scratch directory that is removed before this
/// returns — the sources live in memory from here on, so nothing the import
/// touches outlives it.
pub fn load_snapshot(version: &str, path: &Path) -> Result<VersionedSnapshot, SpecImportError> {
    let files = match snapshot_kind(path) {
        SnapshotKind::Directory => {
            if !path.is_dir() {
                return Err(SpecImportError::SnapshotPath {
                    version: version.to_owned(),
                    path: path.display().to_string(),
                });
            }
            collect_sources(path)
        }
        SnapshotKind::Zip | SnapshotKind::Tarball => {
            let bytes = std::fs::read(path).map_err(|e| SpecImportError::Archive {
                version: version.to_owned(),
                message: format!("cannot read {}: {e}", path.display()),
            })?;
            let scratch = ScratchDir::new("spec-import").map_err(|e| SpecImportError::Archive {
                version: version.to_owned(),
                message: format!("cannot create a scratch directory: {e}"),
            })?;
            let hint = path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            tcl_pkg::fetchers::extract_archive(&bytes, scratch.path(), &hint).map_err(|e| {
                SpecImportError::Archive {
                    version: version.to_owned(),
                    message: e.to_string(),
                }
            })?;
            collect_sources(scratch.path())
        }
    };

    if files.is_empty() {
        return Err(SpecImportError::NoSources {
            version: version.to_owned(),
            path: path.display().to_string(),
            extensions: TCL_EXTENSIONS.join(", "),
        });
    }
    Ok(VersionedSnapshot {
        version: version.to_owned(),
        files,
    })
}

/// Every [`TCL_EXTENSIONS`] file under `root`, named by its path relative to
/// `root` so the evidence notes read the same whatever directory the release
/// was unpacked into.
///
/// Dot-directories are skipped (a checkout's `.git` is not package source) and
/// unreadable or non-UTF-8 files are skipped rather than failing the import:
/// one stray binary in a release tarball is not a reason to derive no ranges.
#[must_use]
pub fn collect_sources(root: &Path) -> Vec<SourceFile> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let lower = name.to_lowercase();
            if !TCL_EXTENSIONS.iter().any(|ext| lower.ends_with(ext)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            files.push(SourceFile {
                name: relative_name(root, &path),
                text,
            });
        }
    }
    // A directory walk's order is the filesystem's; the evidence notes and the
    // rendered pack must not depend on it.
    files.sort_by(|a, b| a.name.cmp(&b.name));
    files
}

/// `path` relative to `root`, always with `/` separators.
fn relative_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Derive the ranges and render the pack.
///
/// The pipeline both front ends share; everything above this line is only
/// about getting bytes off disk.
#[must_use]
pub fn import_snapshots(
    snapshots: &[VersionedSnapshot],
    options: &SpecImportOptions,
) -> SpecImport {
    let import = import_package_versions(
        snapshots,
        options.dialect.name,
        &VersionedImportOptions {
            complete_history: options.complete_history,
        },
    );

    let package = options
        .package
        .map(ToOwned::to_owned)
        .or_else(|| import.package.clone())
        .unwrap_or_else(|| "imported".to_owned());

    let commands: Vec<DerivedCommand> = import
        .commands
        .iter()
        .map(|inferred| DerivedCommand {
            name: inferred.name.clone(),
            introduced_version: draft_version(inferred, "introduced_version"),
            retired_version: draft_version(inferred, "retired_version"),
            notes: inferred.notes.clone(),
        })
        .collect();

    let drafts: Vec<_> = import
        .commands
        .iter()
        .map(|inferred| inferred.draft.clone())
        .collect();
    let (body, losses) = render_pack_reporting(&drafts, &package);
    let losses: Vec<String> = losses
        .into_iter()
        .map(|loss| format!("{}: {}", loss.key, loss.reason))
        .collect();

    let header = header_comments(
        &package,
        &import.versions,
        &commands,
        &import.warnings,
        &losses,
        options,
    );
    SpecImport {
        package,
        versions: import.versions,
        commands,
        warnings: import.warnings,
        losses,
        pack: format!("{header}{body}"),
    }
}

/// A lifecycle field of a merged draft, as a string when the derivation set one.
fn draft_version(inferred: &tcl_spec_studio::infer::Inferred, key: &str) -> Option<String> {
    inferred
        .draft
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

/// The `#` header the pack carries: which releases were analysed, what the
/// caller claimed about them, and every warning, version gate and render loss.
///
/// The evidence has to travel *with* the pack. A range the author cannot audit
/// is worse than no range, and the derivation's reasoning is exactly what a
/// reviewer needs to overrule a bad guess.
fn header_comments(
    package: &str,
    versions: &[String],
    commands: &[DerivedCommand],
    warnings: &[String],
    losses: &[String],
    options: &SpecImportOptions,
) -> String {
    let mut out = String::new();
    let mut line = |text: &str| {
        out.push_str(&comment(text));
    };

    line(&format!(
        "Derived by `tcl spec import` from {} release snapshot(s) of `{package}`, \
         analysed as dialect {}.",
        versions.len(),
        options.dialect.name
    ));
    line(&format!("Releases, oldest first: {}", versions.join(", ")));
    if options.complete_history {
        line(
            "The caller declared this history COMPLETE, so a command present in the \
             earliest release is recorded as introduced there.",
        );
    } else {
        line(
            "The caller did NOT declare this history complete, so presence in the \
             earliest release proves nothing about introduction and \
             `introduced_version` is left unset there. Pass --complete-history when \
             the snapshots really are every release.",
        );
    }

    let gates: Vec<String> = commands
        .iter()
        .flat_map(|c| c.gates().into_iter().map(|g| format!("{}: {g}", c.name)))
        .collect();
    section(&mut out, "Derivation warnings", warnings);
    section(
        &mut out,
        "Version facts no spec field can carry yet (upgrade these by hand)",
        &gates,
    );
    section(&mut out, "Fields this render could not carry", losses);
    // The renderer opens with comments of its own; a rule between the two keeps
    // "what the derivation says" and "what the renderer says" apart.
    out.push_str("# ---\n");
    out
}

/// One `# Heading (n):` block followed by one `#   - item` line each, or
/// nothing at all when there are no items.
fn section(out: &mut String, heading: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    out.push_str("#\n");
    out.push_str(&comment(&format!("{heading} ({}):", items.len())));
    for item in items {
        out.push_str(&comment(&format!("  - {item}")));
    }
}

/// One comment line, made safe to sit in a `.tclspec` file.
///
/// A comment runs to end of line, so an embedded newline would end the comment
/// and leave the rest as a command; a trailing backslash would swallow the
/// next line; and an unbalanced brace or bracket in a comment is a hazard no
/// evidence line is worth. The characters are replaced rather than dropped so
/// the text still reads.
fn comment(text: &str) -> String {
    let mut safe = String::with_capacity(text.len() + 3);
    safe.push_str("# ");
    for ch in text.chars() {
        match ch {
            '\n' | '\r' | '\t' => safe.push(' '),
            '{' | '[' => safe.push('('),
            '}' | ']' => safe.push(')'),
            '\\' => safe.push('/'),
            _ => safe.push(ch),
        }
    }
    safe.push('\n');
    safe
}

/// A scratch directory that removes itself, for unpacking one archive.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(tag: &str) -> std::io::Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("tcl-{tag}-{}-{nanos}-{seq}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_argument_splits_on_the_first_equals() {
        let (version, path) = parse_snapshot_arg("1.2=./rel/1.2").expect("VERSION=PATH");
        assert_eq!(version, "1.2");
        assert_eq!(path, PathBuf::from("./rel/1.2"));

        let (version, path) = parse_snapshot_arg("2.0=/tmp/a=b/pkg.zip").expect("VERSION=PATH");
        assert_eq!(version, "2.0");
        assert_eq!(path, PathBuf::from("/tmp/a=b/pkg.zip"));
    }

    #[test]
    fn a_snapshot_argument_without_both_halves_is_rejected() {
        for bad in ["1.2", "=./rel", "1.2=", "  =x"] {
            assert!(
                parse_snapshot_arg(bad).is_err(),
                "`{bad}` should not parse as VERSION=PATH"
            );
        }
    }

    #[test]
    fn archive_kinds_come_from_the_extension() {
        assert_eq!(snapshot_kind(Path::new("pkg.zip")), SnapshotKind::Zip);
        assert_eq!(snapshot_kind(Path::new("PKG.ZIP")), SnapshotKind::Zip);
        assert_eq!(
            snapshot_kind(Path::new("/a/b/pkg-1.2.tar.gz")),
            SnapshotKind::Tarball
        );
        assert_eq!(snapshot_kind(Path::new("pkg.tgz")), SnapshotKind::Tarball);
        assert_eq!(snapshot_kind(Path::new("pkg.tar")), SnapshotKind::Tarball);
        assert_eq!(
            snapshot_kind(Path::new("releases/1.2")),
            SnapshotKind::Directory
        );
        // `.gz` alone is not a tarball we can walk, and a bare `.zipper`
        // directory must not be mistaken for an archive.
        assert_eq!(
            snapshot_kind(Path::new("pkg.zipper")),
            SnapshotKind::Directory
        );
    }

    #[test]
    fn a_comment_line_cannot_escape_its_line_or_unbalance_the_file() {
        let line = comment("a {brace and a [bracket] and a trailing\\\nsecond line");
        assert!(line.starts_with("# "), "{line}");
        assert_eq!(line.matches('\n').count(), 1, "{line}");
        for hazard in ['{', '}', '[', ']', '\\'] {
            assert!(!line.contains(hazard), "{hazard} survived into {line}");
        }
    }

    #[test]
    fn sources_are_collected_by_extension_in_a_stable_order() {
        let dir = ScratchDir::new("collect-test").expect("scratch dir");
        std::fs::create_dir_all(dir.path().join("sub")).expect("subdir");
        std::fs::create_dir_all(dir.path().join(".git")).expect("dot dir");
        std::fs::write(dir.path().join("b.tcl"), "proc b {} {}\n").expect("write");
        std::fs::write(dir.path().join("a.tm"), "proc a {} {}\n").expect("write");
        std::fs::write(dir.path().join("sub/c.test"), "proc c {} {}\n").expect("write");
        std::fs::write(dir.path().join("README.md"), "not tcl\n").expect("write");
        std::fs::write(dir.path().join(".git/hook.tcl"), "proc h {} {}\n").expect("write");

        let names: Vec<String> = collect_sources(dir.path())
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(names, ["a.tm", "b.tcl", "sub/c.test"]);
    }
}
