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

//! Embedded orchestrator Tcl — the TMM-sim framework bundled into the binary.
//!
//! The simulation is the body of Tcl under `rust/tcl-irule-test/tcl/`. So a
//! consumer (e.g. `f5 explain-flow --simulate`) need not ship that tree
//! alongside the binary, the required framework files are embedded with
//! [`include_str!`] and materialised to a temporary directory on demand. The
//! returned [`EmbeddedLib`] is a directory handle a [`crate::LiveSession`] can
//! be pointed at; it removes the directory when dropped.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// One bundled framework file: its name and contents.
macro_rules! bundled {
    ($name:literal) => {
        ($name, include_str!(concat!("../tcl/", $name)))
    };
}

/// Every framework file the orchestrator needs, materialised together so the
/// `[file dirname [info script]]` sibling lookups (`_registry_data.tcl`,
/// `_event_data.tcl`) resolve. Order is irrelevant on disk — [`crate::LiveSession`]
/// sources them in dependency order.
const BUNDLE: &[(&str, &str)] = &[
    bundled!("compat84.tcl"),
    bundled!("state_layers.tcl"),
    bundled!("tmm_shim.tcl"),
    bundled!("expr_ops.tcl"),
    bundled!("profiler.tcl"),
    bundled!("command_mocks.tcl"),
    bundled!("itest_core.tcl"),
    bundled!("orchestrator.tcl"),
    // Generated data the framework sources transitively via `info script`.
    bundled!("_registry_data.tcl"),
    bundled!("_event_data.tcl"),
    // Generated stub mocks for registry-only iRule commands (e.g.
    // `ACCESS::session`, `AAA::auth`). `LiveSession::bootstrap` sources this
    // before `itest_core.tcl` when present, so an embedded simulator must carry
    // it too — otherwise `f5 explain-flow --simulate` would fail on any iRule
    // using a stub-only command that a repo checkout would have handled.
    bundled!("_mock_stubs.tcl"),
];

/// A temporary directory holding the materialised orchestrator framework. The
/// directory (and its contents) are removed when this handle is dropped.
pub struct EmbeddedLib {
    dir: PathBuf,
}

impl EmbeddedLib {
    /// Write the bundled framework to a fresh temp directory.
    ///
    /// # Errors
    /// Any filesystem error creating the directory or writing a file.
    pub fn materialise() -> io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("tcl-irule-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        for (name, contents) in BUNDLE {
            std::fs::write(dir.join(name), contents)?;
        }
        Ok(Self { dir })
    }

    /// The directory holding `orchestrator.tcl` — pass to [`crate::LiveSession::new`].
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for EmbeddedLib {
    fn drop(&mut self) {
        // Best-effort cleanup; a leftover temp dir is harmless.
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialise_writes_orchestrator() {
        let lib = EmbeddedLib::materialise().expect("materialise");
        assert!(lib.path().join("orchestrator.tcl").is_file());
        assert!(lib.path().join("_registry_data.tcl").is_file());
        let dir = lib.path().to_path_buf();
        drop(lib);
        assert!(!dir.exists(), "temp dir removed on drop");
    }

    /// Guard the embedded framework `.tcl` files against a syntax regression
    /// (the issue-192 `_fakecmp_hash` edit adds an IPv6-aware helper): every
    /// top-level command must be complete — balanced braces / quotes — per the
    /// project's own segmenter.
    #[test]
    fn smoke_bundled_tcl_has_balanced_top_level_commands() {
        for (name, src) in BUNDLE {
            let cmds = tcl_compiler::segmenter::segment_commands(src);
            assert!(
                cmds.iter().all(|c| !c.is_partial),
                "{name} has an unbalanced / partial top-level command (syntax error)"
            );
        }
        // The fakeCMP helper the fix introduces is present in the orchestrator.
        let orch = BUNDLE
            .iter()
            .find(|(n, _)| *n == "orchestrator.tcl")
            .map(|(_, s)| *s)
            .expect("orchestrator bundled");
        assert!(orch.contains("_fakecmp_addr_parts"));
    }

    /// The simulator is a deliberate non-Rust loader carve-out. Keep its
    /// header recogniser narrow and explicitly tied to the canonical Rust
    /// boundary owner; this fails when either side is silently changed.
    #[test]
    fn itest_core_when_loader_contract() {
        let core = BUNDLE
            .iter()
            .find(|(n, _)| *n == "itest_core.tcl")
            .map(|(_, s)| *s)
            .expect("itest core bundled");
        assert!(core.contains("Rust consumers must use `tcl-irules::when_blocks`"));
        assert!(core.contains("when\\s+([A-Z_][A-Z0-9_]*)"));
        assert!(core.contains("Find the body (brace-balanced)"));
    }
}
