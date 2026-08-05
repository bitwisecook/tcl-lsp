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

//! The capability/policy facet of the profile-based top layer.
//!
//! Top-level `allow CMD …` / `deny CMD …` declarations sandbox a program to a
//! declared set of operations. The check runs over the **expanded** handler
//! body (after `use`/field/loop expansion), so a template or profile field
//! cannot smuggle a forbidden operation past the policy.
//!
//! Which verbs the policy governs is **registry data**: every BPF-Tcl command
//! spec carries a typed [`BpfOpSpec`](tcl_registry::bpf_op::BpfOpSpec) whose
//! effect classification says whether the verb is *gated* (touches the packet
//! or a map — `BpfEffects::is_gated`) or a *verdict*
//! (`BpfEffects::is_verdict`). This module never names a command:
//!
//! - `deny X …` forbids each `X` wherever it appears (default-allow otherwise),
//!   and may name a verdict (a profile that inspects but never drops).
//! - `allow X …` restricts the *gated* set: a gated verb not listed is denied.
//!   Verdicts and structural verbs stay allowed unless separately denied.
//! - A verb in both `allow` and `deny` is denied (deny wins).

use tcl_compiler::{Script, Statement};
use tcl_registry::registry::CommandRegistry;

use crate::diag::{BpfDiag, BpfError};

/// Whether `cmd`'s registry descriptor marks it gated (packet/map access).
fn is_gated(cmd: &str, registry: &CommandRegistry) -> bool {
    registry.bpf_op(cmd).is_some_and(|op| op.effects.is_gated())
}

/// Is `cmd` something `allow`/`deny` may name (a gated verb or a verdict)?
fn is_governable(cmd: &str, registry: &CommandRegistry) -> bool {
    registry
        .bpf_op(cmd)
        .is_some_and(|op| op.effects.is_gated() || op.effects.is_verdict())
}

/// A capability policy collected from a file's top-level `allow`/`deny` decls.
#[derive(Debug, Clone, Default)]
pub struct CapabilityPolicy {
    /// The allowlist of gated verbs, if any `allow` was declared. `None` means
    /// "no allowlist" (gated verbs are permitted unless denied).
    allow: Option<Vec<String>>,
    /// Verbs explicitly denied.
    deny: Vec<String>,
}

impl CapabilityPolicy {
    /// Does this policy permit `cmd`? `gated` is the verb's registry effect
    /// classification (see [`is_gated`]).
    #[must_use]
    pub fn permits(&self, cmd: &str, gated: bool) -> bool {
        if self.deny.iter().any(|d| d == cmd) {
            return false;
        }
        if let Some(allow) = &self.allow
            && gated
        {
            return allow.iter().any(|a| a == cmd);
        }
        true
    }

    /// Is this policy a no-op (no `allow`, no `deny`)? Lets the front-end skip
    /// the walk entirely.
    #[must_use]
    pub fn is_unrestricted(&self) -> bool {
        self.allow.is_none() && self.deny.is_empty()
    }
}

/// Collect the capability policy from a file's top-level `allow`/`deny` decls.
///
/// # Errors
/// A bad arity, or a verb that is not capability-governable (likely a typo).
pub fn collect_policy(
    top_level: &Script,
    registry: &CommandRegistry,
) -> Result<CapabilityPolicy, BpfError> {
    let mut policy = CapabilityPolicy::default();
    for stmt in &top_level.statements {
        let Statement::Call {
            command,
            canonical_command,
            args,
            span,
            ..
        } = stmt
        else {
            continue;
        };
        let cmd = canonical_command.as_deref().unwrap_or(command.as_str());
        let kind = match cmd {
            "allow" => true,
            "deny" => false,
            _ => continue,
        };
        if args.is_empty() {
            return Err(BpfError::new(
                BpfDiag::CapabilityDenied,
                *span,
                format!("`{cmd}` expects: {cmd} CMD ?CMD …?"),
            ));
        }
        for verb in args {
            if !is_governable(verb, registry) {
                return Err(BpfError::new(
                    BpfDiag::CapabilityDenied,
                    *span,
                    format!(
                        "`{verb}` is not a capability-governable command \
                         (governable: {})",
                        governable_list(registry)
                    ),
                ));
            }
            if kind {
                policy.allow.get_or_insert_with(Vec::new).push(verb.clone());
            } else {
                policy.deny.push(verb.clone());
            }
        }
    }
    Ok(policy)
}

/// Every governable verb name, from the registry descriptors (sorted).
fn governable_list(registry: &CommandRegistry) -> String {
    let mut all: Vec<&str> = registry
        .command_names()
        .filter(|name| is_governable(name, registry))
        .collect();
    all.sort_unstable();
    all.dedup();
    all.join(", ")
}

/// Enforce `policy` over an expanded handler body, rejecting the first verb the
/// policy forbids. Recurses into `if` bodies.
///
/// # Errors
/// [`BpfDiag::CapabilityDenied`] at the offending command's span.
pub fn check_policy(
    script: &Script,
    policy: &CapabilityPolicy,
    registry: &CommandRegistry,
) -> Result<(), BpfError> {
    if policy.is_unrestricted() {
        return Ok(());
    }
    check_script(script, policy, registry)
}

fn check_script(
    script: &Script,
    policy: &CapabilityPolicy,
    registry: &CommandRegistry,
) -> Result<(), BpfError> {
    for stmt in &script.statements {
        match stmt {
            Statement::Call {
                command,
                canonical_command,
                span,
                ..
            } => {
                let cmd = canonical_command.as_deref().unwrap_or(command.as_str());
                if !policy.permits(cmd, is_gated(cmd, registry)) {
                    return Err(BpfError::new(
                        BpfDiag::CapabilityDenied,
                        *span,
                        format!("`{cmd}` is forbidden by the profile's capability policy"),
                    ));
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    check_script(&c.body, policy, registry)?;
                }
                if let Some(b) = else_body {
                    check_script(b, policy, registry)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
