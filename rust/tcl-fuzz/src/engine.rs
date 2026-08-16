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

//! Named backend engines the differential harness can pair up (issue #1313).
//!
//! Before this, `tcl-fuzz` only ever compared one fixed pair: `tclvm`
//! (subject) against `tclsh` (reference). [`Engine`] generalises "a backend
//! the harness can run a script through" so any two engines can be paired —
//! `runtime/rust` ↔ `tclsh`, `tclvm` ↔ `runtime/rust`, or the original
//! `tclvm` ↔ `tclsh` — over the *same* subprocess harness (`harness.rs`).

use std::path::{Path, PathBuf};

/// A backend the fuzzer can run a generated script through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Engine {
    /// The native bytecode VM CLI (`tclvm`).
    Tclvm,
    /// A reference `tclsh` (a C Tcl interpreter).
    Tclsh,
    /// `runtime/rust`'s tree-walking interpreter, driven through its
    /// `run_script` dev-tool example binary (`--quiet` so its stdout contract
    /// matches `tclsh script.tcl`'s — see that binary's doc comment).
    RuntimeRust,
}

impl Engine {
    /// The engine's short label — used in CLI help, findings-directory
    /// namespacing (so two backend pairs never collide on a shared seed
    /// directory), and log output.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Tclvm => "tclvm",
            Self::Tclsh => "tclsh",
            Self::RuntimeRust => "runtime-rust",
        }
    }

    /// Fixed leading arguments this engine's binary needs before the script
    /// path (`harness::run_backend` appends the script path after these).
    #[must_use]
    pub fn args(self) -> Vec<String> {
        match self {
            Self::Tclvm | Self::Tclsh => Vec::new(),
            // `run_script`'s own result-echo would otherwise show up as a
            // spurious stdout divergence on any script whose last command
            // leaves a non-empty result — `tclsh script.tcl` never echoes it.
            Self::RuntimeRust => vec!["--quiet".to_string()],
        }
    }

    /// Arguments that select this engine's emulated Tcl release.
    ///
    /// A C `tclsh` binary is already one fixed release, so it deliberately
    /// receives no runtime argument. The pair probe must subsequently verify
    /// that the selected binary reports the requested release; an unmatched
    /// binary is rejected before the campaign begins.
    #[must_use]
    pub fn tcl_version_args(self, version: tcl_dialect::TclVersion) -> Vec<String> {
        match self {
            Self::Tclvm | Self::RuntimeRust => vec![
                "--tcl-version".to_owned(),
                version.version_string().to_owned(),
            ],
            Self::Tclsh => Vec::new(),
        }
    }

    /// Locate this engine's binary: an explicit override (e.g. `--tclvm`,
    /// `--tclsh`, `--runtime-rust`) first, else a convention-based search.
    /// `None` means genuinely not found, and the caller refuses to start the
    /// campaign — naming the engine and the flag to pass. `tclsh` never
    /// returns `None`: it falls back to the bare name so an absent reference
    /// surfaces per-script as [`crate::harness::Outcome::Unavailable`]
    /// (skipped, not a finding), which is the long-standing behaviour.
    #[must_use]
    pub fn resolve(self, explicit: Option<&Path>) -> Option<PathBuf> {
        if let Some(p) = explicit {
            return Some(p.to_owned());
        }
        match self {
            Self::Tclvm => beside_this_exe("tclvm").or_else(|| which("tclvm")),
            Self::Tclsh => which("tclsh9.0")
                .or_else(|| which("tclsh"))
                .or(Some(PathBuf::from("tclsh"))),
            Self::RuntimeRust => beside_this_exe("run_script").or_else(|| which("run_script")),
        }
    }
}

/// A binary named `name` beside this executable (the convention `tclvm`'s
/// resolution already used, e.g. when `tcl-fuzz` and `tclvm` are both dropped
/// into the same `target/.../` directory by a build script).
fn beside_this_exe(name: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let beside = dir.join(name);
    beside.is_file().then_some(beside)
}

/// Find an executable on `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_rust_carries_the_quiet_flag() {
        assert_eq!(Engine::RuntimeRust.args(), vec!["--quiet".to_string()]);
    }

    #[test]
    fn tclvm_and_tclsh_carry_no_extra_args() {
        assert!(Engine::Tclvm.args().is_empty());
        assert!(Engine::Tclsh.args().is_empty());
    }

    #[test]
    fn emulated_engines_accept_a_release_flag_but_tclsh_uses_its_selected_binary() {
        let expected = vec!["--tcl-version".to_owned(), "8.6".to_owned()];
        assert_eq!(
            Engine::Tclvm.tcl_version_args(tcl_dialect::TclVersion::V8_6),
            expected.clone(),
        );
        assert_eq!(
            Engine::RuntimeRust.tcl_version_args(tcl_dialect::TclVersion::V8_6),
            expected,
        );
        assert!(
            Engine::Tclsh
                .tcl_version_args(tcl_dialect::TclVersion::V8_6)
                .is_empty()
        );
    }

    #[test]
    fn labels_are_distinct_for_findings_namespacing() {
        let labels = [
            Engine::Tclvm.label(),
            Engine::Tclsh.label(),
            Engine::RuntimeRust.label(),
        ];
        for (i, a) in labels.iter().enumerate() {
            for (j, b) in labels.iter().enumerate() {
                assert!(i == j || a != b, "labels must be pairwise distinct");
            }
        }
    }

    #[test]
    fn explicit_override_wins_regardless_of_convention() {
        let p = Path::new("/does/not/exist/custom-tclsh");
        assert_eq!(Engine::Tclsh.resolve(Some(p)), Some(p.to_owned()));
    }
}
