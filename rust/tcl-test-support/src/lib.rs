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

//! Shared discovery and raw execution support for C Tcl conformance oracles.
//!
//! Test crates and developer tools use the same release matrix, environment
//! variables, binary validation, and source-tree validation. Semantic tests
//! remain in their owning crates; this crate only owns how they find and run
//! the reference implementation.

use std::fmt;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tcl_dialect::TclVersion;

/// A successfully validated Tcl interpreter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tclsh {
    /// Release line reported by `info tclversion`.
    pub version: TclVersion,
    /// Full version reported by `info patchlevel`.
    pub patchlevel: String,
    /// Executable path or PATH-resolved command name.
    pub path: PathBuf,
}

/// A validated upstream Tcl source tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TclSourceTree {
    /// Full version read from `generic/tcl.h`.
    pub patchlevel: String,
    /// Root containing `generic/`, `library/`, and `tests/`.
    pub root: PathBuf,
}

impl TclSourceTree {
    #[must_use]
    pub fn tests_dir(&self) -> PathBuf {
        self.root.join("tests")
    }

    #[must_use]
    pub fn library_dir(&self) -> PathBuf {
        self.root.join("library")
    }
}

/// Raw process outcome. Byte channels stay byte-valued so an oracle cannot
/// accidentally bless a lossy UTF-8 conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptOutcome {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ScriptOutcome {
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Strict text projection used by legacy string-valued conformance tests.
    /// A Tcl error written to stderr is an error even when interactive stdin
    /// evaluation leaves the process exit code at zero.
    pub fn strict_text(&self) -> Result<String, String> {
        if !self.success() || !self.stderr.is_empty() {
            let complaint = String::from_utf8_lossy(&self.stderr).trim().to_owned();
            return if complaint.is_empty() {
                Err(format!("tclsh exited with {:?}", self.exit_code))
            } else {
                Err(complaint)
            };
        }
        String::from_utf8(self.stdout.clone())
            .map(|text| text.trim().to_owned())
            .map_err(|error| format!("tclsh stdout is not UTF-8: {error}"))
    }
}

#[derive(Debug)]
pub enum OracleError {
    Io {
        action: &'static str,
        source: std::io::Error,
    },
    InvalidOverride(String),
    InvalidInterpreter(String),
    InvalidSourceTree(String),
}

impl fmt::Display for OracleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { action, source } => write!(formatter, "{action}: {source}"),
            Self::InvalidOverride(message)
            | Self::InvalidInterpreter(message)
            | Self::InvalidSourceTree(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for OracleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidOverride(_) | Self::InvalidInterpreter(_) | Self::InvalidSourceTree(_) => {
                None
            }
        }
    }
}

struct ReleaseLocation {
    version: TclVersion,
    binary_env: &'static str,
    source_env: &'static str,
    binary_names: &'static [&'static str],
}

const RELEASES: &[ReleaseLocation] = &[
    ReleaseLocation {
        version: TclVersion::V8_4,
        binary_env: "TCL_LSP_TCLSH84",
        source_env: "TCL_LSP_TCL_ROOT84",
        binary_names: &["tclsh8.4"],
    },
    ReleaseLocation {
        version: TclVersion::V8_5,
        binary_env: "TCL_LSP_TCLSH85",
        source_env: "TCL_LSP_TCL_ROOT85",
        binary_names: &["tclsh8.5"],
    },
    ReleaseLocation {
        version: TclVersion::V8_6,
        binary_env: "TCL_LSP_TCLSH86",
        source_env: "TCL_LSP_TCL_ROOT86",
        binary_names: &["tclsh8.6"],
    },
    ReleaseLocation {
        version: TclVersion::V9_0,
        binary_env: "TCL_LSP_TCLSH90",
        source_env: "TCL_LSP_TCL_ROOT90",
        binary_names: &["tclsh9.0"],
    },
    ReleaseLocation {
        version: TclVersion::V9_1,
        binary_env: "TCL_LSP_TCLSH91",
        source_env: "TCL_LSP_TCL_ROOT91",
        binary_names: &["tclsh9.1"],
    },
];

fn release_location(version: TclVersion) -> &'static ReleaseLocation {
    RELEASES
        .iter()
        .find(|entry| entry.version == version)
        .expect("every TclVersion has an oracle location")
}

/// Locate one release's interpreter. An explicit environment override is a
/// promise: a missing or wrongly-versioned override fails instead of silently
/// falling through to a different binary.
pub fn locate_tclsh(version: TclVersion) -> Result<Option<Tclsh>, OracleError> {
    let location = release_location(version);
    if let Some(explicit) = std::env::var_os(location.binary_env) {
        let path = PathBuf::from(explicit);
        if !path.is_file() {
            return Err(OracleError::InvalidOverride(format!(
                "{}={} does not point at a file",
                location.binary_env,
                path.display()
            )));
        }
        return validate_tclsh(path, version).map(Some);
    }
    for name in location.binary_names {
        if let Some(path) = which_on_path(name)
            && let Ok(interpreter) = validate_tclsh(path, version)
        {
            return Ok(Some(interpreter));
        }
    }
    Ok(None)
}

/// Validate the interpreter built inside `source_tree` and require its exact
/// patch level to match the source headers. Conformance harnesses use this when
/// the interpreter and library must be one indivisible oracle selection.
pub fn tclsh_from_source_tree(
    source_tree: &TclSourceTree,
    version: TclVersion,
) -> Result<Tclsh, OracleError> {
    let path = source_tree.root.join("unix/tclsh");
    if !path.is_file() {
        return Err(OracleError::InvalidInterpreter(format!(
            "{} has no built unix/tclsh",
            source_tree.root.display()
        )));
    }
    let interpreter =
        validate_tclsh_with_library(path, version, Some(source_tree.root.join("unix").as_path()))?;
    if interpreter.patchlevel != source_tree.patchlevel {
        return Err(OracleError::InvalidInterpreter(format!(
            "{} reports Tcl {}, but source tree {} is Tcl {}",
            interpreter.path.display(),
            interpreter.patchlevel,
            source_tree.root.display(),
            source_tree.patchlevel
        )));
    }
    Ok(interpreter)
}

/// Every available reference interpreter, in release order.
#[must_use]
pub fn available_tclshs() -> Vec<Tclsh> {
    let mut interpreters = Vec::new();
    for version in TclVersion::ALL {
        match locate_tclsh(version) {
            Ok(Some(interpreter)) => interpreters.push(interpreter),
            Ok(None) => eprintln!(
                "skipping Tcl {}: no interpreter (set {})",
                version.version_string(),
                release_location(version).binary_env
            ),
            Err(error) => eprintln!("skipping Tcl {}: {error}", version.version_string()),
        }
    }
    interpreters
}

/// Run a Tcl script through a reference interpreter and preserve its raw byte
/// channels and exit code.
pub fn run_script(tclsh: &Path, script: &[u8]) -> Result<ScriptOutcome, OracleError> {
    run_script_with_library(tclsh, script, None)
}

fn run_script_with_library(
    tclsh: &Path,
    script: &[u8],
    library_dir: Option<&Path>,
) -> Result<ScriptOutcome, OracleError> {
    let mut command = Command::new(tclsh);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(library_dir) = library_dir {
        let mut paths = vec![library_dir.to_owned()];
        if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        let joined = std::env::join_paths(paths).map_err(|source| OracleError::Io {
            action: "constructing LD_LIBRARY_PATH",
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, source),
        })?;
        command.env("LD_LIBRARY_PATH", joined);
    }
    let mut child = command.spawn().map_err(|source| OracleError::Io {
        action: "spawning tclsh",
        source,
    })?;
    child
        .stdin
        .as_mut()
        .expect("piped tclsh stdin")
        .write_all(script)
        .map_err(|source| OracleError::Io {
            action: "writing tclsh script",
            source,
        })?;
    let output = child.wait_with_output().map_err(|source| OracleError::Io {
        action: "waiting for tclsh",
        source,
    })?;
    Ok(ScriptOutcome {
        exit_code: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

/// Locate a release's source tree. Resolution order is explicit CLI path,
/// release-specific environment override, the repository's pinned `tmp/`
/// tree, then matching trees under the checkout parent and `$HOME/src`.
pub fn locate_source_tree(
    repo_root: &Path,
    version: TclVersion,
    explicit: Option<&Path>,
) -> Result<Option<TclSourceTree>, OracleError> {
    let location = release_location(version);
    if let Some(path) = explicit {
        return validate_source_tree(path, version).map(Some);
    }
    if let Some(value) = std::env::var_os(location.source_env) {
        let path = PathBuf::from(value);
        return validate_source_tree(&path, version)
            .map(Some)
            .map_err(|error| {
                OracleError::InvalidOverride(format!(
                    "{}={} is invalid: {error}",
                    location.source_env,
                    path.display()
                ))
            });
    }

    let pinned = repo_root
        .join("tmp")
        .join(format!("tcl{}", version.patchlevel()));
    if let Ok(tree) = validate_source_tree(&pinned, version) {
        return Ok(Some(tree));
    }

    let mut candidates = Vec::new();
    if let Some(parent) = repo_root.parent() {
        collect_source_candidates(parent, version, &mut candidates);
    }
    if let Some(home) = std::env::var_os("HOME") {
        collect_source_candidates(&PathBuf::from(home).join("src"), version, &mut candidates);
    }
    candidates.sort_by(|left, right| right.cmp(left));
    candidates.dedup();
    for candidate in candidates {
        if let Ok(tree) = validate_source_tree(&candidate, version) {
            return Ok(Some(tree));
        }
    }
    Ok(None)
}

fn validate_tclsh(path: PathBuf, version: TclVersion) -> Result<Tclsh, OracleError> {
    validate_tclsh_with_library(path, version, None)
}

fn validate_tclsh_with_library(
    path: PathBuf,
    version: TclVersion,
    library_dir: Option<&Path>,
) -> Result<Tclsh, OracleError> {
    let outcome = run_script_with_library(
        &path,
        b"puts [info tclversion]\nputs [info patchlevel]\n",
        library_dir,
    )?;
    let text = outcome.strict_text().map_err(|message| {
        OracleError::InvalidInterpreter(format!(
            "{} is not a usable tclsh: {message}",
            path.display()
        ))
    })?;
    let mut lines = text.lines();
    let reported_version = lines.next().unwrap_or_default();
    let patchlevel = lines.next().unwrap_or_default().to_owned();
    if reported_version != version.version_string() || patchlevel.is_empty() {
        return Err(OracleError::InvalidInterpreter(format!(
            "{} reports Tcl {reported_version} {patchlevel}, expected {}",
            path.display(),
            version.version_string()
        )));
    }
    Ok(Tclsh {
        version,
        patchlevel,
        path,
    })
}

fn validate_source_tree(path: &Path, version: TclVersion) -> Result<TclSourceTree, OracleError> {
    for required in ["generic/tcl.h", "library/init.tcl", "tests/all.tcl"] {
        if !path.join(required).is_file() {
            return Err(OracleError::InvalidSourceTree(format!(
                "{} is not a Tcl source tree: missing {required}",
                path.display()
            )));
        }
    }
    let header =
        fs::read_to_string(path.join("generic/tcl.h")).map_err(|source| OracleError::Io {
            action: "reading generic/tcl.h",
            source,
        })?;
    let patchlevel = patchlevel_from_header(&header).ok_or_else(|| {
        OracleError::InvalidSourceTree(format!(
            "{} has no TCL_PATCH_LEVEL in generic/tcl.h",
            path.display()
        ))
    })?;
    if !patchlevel.starts_with(&format!("{}.", version.version_string()))
        && patchlevel != version.version_string()
        && !patchlevel.starts_with(&format!("{}a", version.version_string()))
        && !patchlevel.starts_with(&format!("{}b", version.version_string()))
    {
        return Err(OracleError::InvalidSourceTree(format!(
            "{} contains Tcl {patchlevel}, expected release {}",
            path.display(),
            version.version_string()
        )));
    }
    Ok(TclSourceTree {
        patchlevel,
        root: path.to_path_buf(),
    })
}

fn patchlevel_from_header(header: &str) -> Option<String> {
    header.lines().find_map(|line| {
        let line = line.trim();
        if !line.starts_with('#') || !line.contains("define TCL_PATCH_LEVEL") {
            return None;
        }
        let (_, value) = line.split_once("TCL_PATCH_LEVEL")?;
        let value = value.trim();
        value
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .map(ToOwned::to_owned)
    })
}

fn collect_source_candidates(parent: &Path, version: TclVersion, out: &mut Vec<PathBuf>) {
    let prefix = format!("tcl{}", version.version_string());
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    out.extend(entries.filter_map(Result::ok).filter_map(|entry| {
        let name = entry.file_name();
        name.to_str()
            .filter(|text| text.starts_with(&prefix))
            .map(|_| entry.path())
    }));
}

fn which_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::{patchlevel_from_header, validate_source_tree};
    use std::fs;
    use std::path::PathBuf;
    use tcl_dialect::TclVersion;

    #[test]
    fn patchlevel_is_read_from_the_c_header() {
        let header = "#   define TCL_VERSION \"9.0\"\n#   define TCL_PATCH_LEVEL \"9.0.4\"\n";
        assert_eq!(patchlevel_from_header(header).as_deref(), Some("9.0.4"));
    }

    #[test]
    fn source_tree_validation_checks_release_and_layout() {
        let root = fixture_root("source-tree");
        fs::create_dir_all(root.join("generic")).expect("generic directory");
        fs::create_dir_all(root.join("library")).expect("library directory");
        fs::create_dir_all(root.join("tests")).expect("tests directory");
        fs::write(
            root.join("generic/tcl.h"),
            "# define TCL_PATCH_LEVEL \"9.0.3\"\n",
        )
        .expect("version header");
        fs::write(root.join("library/init.tcl"), "").expect("init.tcl");
        fs::write(root.join("tests/all.tcl"), "").expect("all.tcl");

        let tree = validate_source_tree(&root, TclVersion::V9_0).expect("valid tree");
        assert_eq!(tree.patchlevel, "9.0.3");
        assert!(validate_source_tree(&root, TclVersion::V8_6).is_err());
        fs::remove_dir_all(root).expect("remove fixture");
    }

    fn fixture_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("tcl-test-support-{tag}-{}", std::process::id()))
    }
}
