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

//! Operator lifecycle hooks.
//!
//! Hooks are defined in policy (so they live in the locked-down system layer and
//! a developer cannot remove them) and fire at well-defined points in a package
//! operation. A hook is an external command run through the [`crate::exec`]
//! sandbox with the capabilities it declares, clamped by the sandbox policy
//! floor. **A non-zero exit aborts the operation** — this is how an operator's
//! scanner, denylist, cooldown check or provenance verifier blocks a package.
//!
//! Context (package name, version, source URL, on-disk path, …) is passed to the
//! hook as scrubbed `TCLPKG_*` environment variables, and `${VAR}` references in
//! a hook's `command`/`fs-read`/`fs-write` entries are expanded from it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use tcl_sandbox::Profile;

use crate::errors::{Category, TclPkgError};
use crate::policy::{HookDef, PolicyConfig};

/// A lifecycle stage at which hooks may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    PreResolve,
    PostResolve,
    PreFetch,
    PostFetch,
    PreBuild,
    PostBuild,
    PreInstall,
    PostInstall,
    PreRun,
    PostRun,
}

impl Stage {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Stage::PreResolve => "pre-resolve",
            Stage::PostResolve => "post-resolve",
            Stage::PreFetch => "pre-fetch",
            Stage::PostFetch => "post-fetch",
            Stage::PreBuild => "pre-build",
            Stage::PostBuild => "post-build",
            Stage::PreInstall => "pre-install",
            Stage::PostInstall => "post-install",
            Stage::PreRun => "pre-run",
            Stage::PostRun => "post-run",
        }
    }

    /// Parse a stage name as written in policy TOML.
    #[must_use]
    pub fn parse(s: &str) -> Option<Stage> {
        Some(match s {
            "pre-resolve" => Stage::PreResolve,
            "post-resolve" => Stage::PostResolve,
            "pre-fetch" => Stage::PreFetch,
            "post-fetch" => Stage::PostFetch,
            "pre-build" => Stage::PreBuild,
            "post-build" => Stage::PostBuild,
            "pre-install" => Stage::PreInstall,
            "post-install" => Stage::PostInstall,
            "pre-run" => Stage::PreRun,
            "post-run" => Stage::PostRun,
            _ => return None,
        })
    }
}

/// The per-invocation context handed to hooks.
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    /// The directory the hook runs in (and is confined to).
    pub cwd: PathBuf,
    /// `TCLPKG_*` variables exported to the hook and used for `${VAR}` expansion.
    pub vars: HashMap<String, String>,
}

impl HookContext {
    #[must_use]
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            vars: HashMap::new(),
        }
    }

    #[must_use]
    pub fn var(mut self, key: &str, value: impl Into<String>) -> Self {
        self.vars.insert(format!("TCLPKG_{key}"), value.into());
        self
    }
}

/// Run every hook bound to `stage`, in declaration order. Aborts on the first
/// hook that fails.
///
/// # Errors
///
/// Returns a [`TclPkgError`] (category [`Category::Hook`]) when a hook is
/// misconfigured, the sandbox refuses to run it, or the hook exits non-zero.
pub fn run_stage(stage: Stage, cfg: &PolicyConfig, ctx: &HookContext) -> Result<(), TclPkgError> {
    let policy = cfg.sandbox_policy();
    for hook in &cfg.hooks {
        if Stage::parse(&hook.stage) != Some(stage) {
            continue;
        }
        run_one(hook, stage, &policy, ctx)?;
    }
    Ok(())
}

fn run_one(
    hook: &HookDef,
    stage: Stage,
    policy: &tcl_sandbox::SandboxPolicy,
    ctx: &HookContext,
) -> Result<(), TclPkgError> {
    let Some((program, args)) = hook.command.split_first() else {
        return Err(
            TclPkgError::new(format!("hook '{}' has an empty command", hook.name))
                .with_category(Category::Hook),
        );
    };
    let program = expand(program, &ctx.vars);

    let mut profile = Profile::new(
        format!("hook:{}:{}", stage.as_str(), hook.name),
        PathBuf::from(&program),
        &ctx.cwd,
    )
    .args(args.iter().map(|a| expand(a, &ctx.vars)))
    .network(hook.network)
    .timeout(hook.timeout_secs.map(Duration::from_secs));

    for (k, v) in &ctx.vars {
        profile = profile.set_env(k, v);
    }
    for name in &hook.env {
        profile = profile.pass_env(name);
    }
    profile.fs_read = hook
        .fs_read
        .iter()
        .map(|p| PathBuf::from(expand(p, &ctx.vars)))
        .collect();
    profile.fs_write = hook
        .fs_write
        .iter()
        .map(|p| PathBuf::from(expand(p, &ctx.vars)))
        .collect();

    let outcome = crate::exec::execute(&profile, policy)?;
    if outcome.success {
        return Ok(());
    }

    let detail = first_nonempty_line(&outcome.stderr)
        .or_else(|| first_nonempty_line(&outcome.stdout))
        .unwrap_or_default();
    let reason = if outcome.timed_out {
        "timed out".to_string()
    } else {
        match outcome.code {
            Some(c) => format!("exited with code {c}"),
            None => "killed".to_string(),
        }
    };
    let mut msg = format!("hook '{}' ({}) {reason}", hook.name, stage.as_str());
    if !detail.is_empty() {
        msg.push_str(": ");
        msg.push_str(&detail);
    }
    Err(TclPkgError::new(msg)
        .with_category(Category::Hook)
        .with_hint("the operator hook rejected this operation"))
}

/// Expand `${NAME}` references from `vars`; unknown names expand to empty.
///
/// Operates on `&str` slices (split at ASCII `${` / `}` boundaries, which are
/// always char boundaries) rather than re-encoding each byte as a `char`, which
/// mojibake-corrupted non-ASCII text — a hook `command = ["/opt/prüfer"]`
/// became `/opt/prÃ¼fer`.
fn expand(input: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(dollar) = rest.find("${") {
        out.push_str(&rest[..dollar]);
        let after = &rest[dollar + 2..];
        if let Some(end) = after.find('}') {
            if let Some(v) = vars.get(&after[..end]) {
                out.push_str(v);
            }
            rest = &after[end + 1..];
        } else {
            // No closing `}` — the `${` is literal text.
            out.push_str("${");
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn first_nonempty_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(ToString::to_string)
}

/// List the configured hooks grouped for display, in stage order.
#[must_use]
pub fn describe(cfg: &PolicyConfig) -> Vec<(String, String, String)> {
    let mut rows = Vec::new();
    for hook in &cfg.hooks {
        let cmd = hook.command.join(" ");
        rows.push((hook.stage.clone(), hook.name.clone(), cmd));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_round_trips() {
        for s in [
            Stage::PreFetch,
            Stage::PostFetch,
            Stage::PreInstall,
            Stage::PostRun,
        ] {
            assert_eq!(Stage::parse(s.as_str()), Some(s));
        }
        assert_eq!(Stage::parse("nope"), None);
    }

    #[test]
    fn expand_substitutes_known_vars() {
        let mut vars = HashMap::new();
        vars.insert("TCLPKG_PKG_DIR".to_string(), "/cache/foo".to_string());
        assert_eq!(expand("${TCLPKG_PKG_DIR}/x", &vars), "/cache/foo/x");
        assert_eq!(expand("${MISSING}/y", &vars), "/y");
        assert_eq!(expand("no-vars", &vars), "no-vars");
    }

    #[test]
    fn expand_preserves_non_ascii() {
        // Non-ASCII text must survive verbatim, not be
        // re-encoded byte-by-byte as Latin-1 (`prüfer` → `prÃ¼fer`).
        let mut vars = HashMap::new();
        vars.insert("DIR".to_string(), "/opt/prüfer".to_string());
        assert_eq!(expand("/opt/prüfer/bin", &vars), "/opt/prüfer/bin");
        assert_eq!(expand("${DIR}/café", &vars), "/opt/prüfer/café");
        // An unterminated `${` is literal (no closing brace).
        assert_eq!(expand("a${B c", &vars), "a${B c");
        // Emoji / multi-byte around a substitution.
        assert_eq!(expand("→${MISSING}←", &vars), "→←");
    }

    #[test]
    fn empty_command_is_rejected() {
        let hook = HookDef {
            name: "bad".into(),
            stage: "post-fetch".into(),
            command: vec![],
            ..HookDef::default()
        };
        let err = run_one(
            &hook,
            Stage::PostFetch,
            &tcl_sandbox::SandboxPolicy::default(),
            &HookContext::new("."),
        )
        .unwrap_err();
        assert_eq!(err.category, Category::Hook);
    }
}
