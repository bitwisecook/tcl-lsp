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

//! The single chokepoint every external execution flows through.
//!
//! Nothing in `tcl-pkg` spawns a process directly: `git`, `tclsh`, operator
//! hooks and package build scripts all go through [`execute`], which applies the
//! sandbox policy and appends an audit record. Routing every exec through one
//! place is what makes "everything the package manager runs is sandboxed" a
//! property we can actually uphold and review.

use std::io::Write;

use tcl_sandbox::{Outcome, Profile, SandboxPolicy};

use crate::errors::{Category, TclPkgError};

/// Run `profile` under `policy`, recording the result to the audit log.
///
/// # Errors
///
/// Returns a [`TclPkgError`] (category [`Category::Sandbox`]) when the sandbox
/// refuses to run the command (fail-closed) or the child cannot be spawned.
pub fn execute(profile: &Profile, policy: &SandboxPolicy) -> Result<Outcome, TclPkgError> {
    let result = tcl_sandbox::run(profile, policy);
    match &result {
        Ok(outcome) => audit(profile, Some(outcome), None),
        Err(e) => audit(profile, None, Some(&e.to_string())),
    }
    result.map_err(|e| {
        TclPkgError::new(e.to_string())
            .with_category(Category::Sandbox)
            .with_hint("adjust the sandbox policy or grant the required capability")
    })
}

/// Append a one-line JSON audit record. Best-effort: failures to write the log
/// never block package operations.
fn audit(profile: &Profile, outcome: Option<&Outcome>, error: Option<&str>) {
    let mut obj = serde_json::Map::new();
    obj.insert("ts".into(), chrono::Utc::now().to_rfc3339().into());
    obj.insert("label".into(), profile.label.clone().into());
    obj.insert(
        "program".into(),
        profile.program.to_string_lossy().into_owned().into(),
    );
    obj.insert(
        "network_requested".into(),
        serde_json::Value::Bool(profile.allow_network),
    );
    if let Some(o) = outcome {
        obj.insert("isolation".into(), o.isolation.as_str().into());
        obj.insert(
            "network_enforced".into(),
            serde_json::Value::Bool(o.network_enforced),
        );
        obj.insert("success".into(), serde_json::Value::Bool(o.success));
        obj.insert("timed_out".into(), serde_json::Value::Bool(o.timed_out));
        if let Some(code) = o.code {
            obj.insert("code".into(), code.into());
        }
    }
    if let Some(e) = error {
        obj.insert("error".into(), e.into());
    }
    let line = serde_json::Value::Object(obj).to_string();

    let dir = crate::state_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("pkg-audit.log"))
    {
        let _ = writeln!(f, "{line}");
    }
}

/// Path to the audit log (may not exist yet).
#[must_use]
pub fn audit_log_path() -> std::path::PathBuf {
    crate::state_dir().join("pkg-audit.log")
}
