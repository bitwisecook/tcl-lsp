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
use tcl_registry::CanonicalCommandDescriptorKey;
use tcl_registry::registry::CommandRegistry;

use crate::diag::{BpfDiag, BpfError};
use crate::source::{OriginalBpfSource, selected_command};

/// Is `cmd` something `allow`/`deny` may name (a gated verb or a verdict)?
fn is_governable(cmd: &str, registry: &CommandRegistry) -> bool {
    registry
        .get_for_surface(cmd, registry.own_surface_query())
        .and_then(|spec| spec.bpf_op)
        .is_some_and(|op| op.effects.is_gated() || op.effects.is_verdict())
}

/// A capability policy collected from a file's top-level `allow`/`deny` decls.
#[derive(Debug, Clone, Default)]
pub struct CapabilityPolicy {
    /// The allowlist of gated verbs, if any `allow` was declared. `None` means
    /// "no allowlist" (gated verbs are permitted unless denied).
    allow: Option<Vec<CanonicalCommandDescriptorKey<'static>>>,
    /// Verbs explicitly denied.
    deny: Vec<CanonicalCommandDescriptorKey<'static>>,
}

impl CapabilityPolicy {
    /// Permit the selected descriptor under its Registry effect classification.
    #[must_use]
    pub fn permits(&self, key: CanonicalCommandDescriptorKey<'_>, gated: bool) -> bool {
        if self.deny.iter().any(|descriptor| *descriptor == key) {
            return false;
        }
        if let Some(allow) = &self.allow
            && gated
        {
            return allow.iter().any(|descriptor| *descriptor == key);
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
/// Missing original static operands, a bad arity, or a verb that is not
/// capability-governable (likely a typo).
pub fn collect_policy(
    top_level: &Script,
    registry: &CommandRegistry,
    source: &OriginalBpfSource,
) -> Result<CapabilityPolicy, BpfError> {
    source.validate_script(top_level, registry, BpfDiag::CapabilityDenied)?;
    let mut policy = CapabilityPolicy::default();
    for stmt in &top_level.statements {
        let declaration = source.declaration_kind(stmt, registry, BpfDiag::CapabilityDenied)?;
        let Statement::Call {
            command,
            args,
            span,
            tokens,
            ..
        } = stmt
        else {
            continue;
        };
        let cmd = command;
        let kind = match declaration {
            Some(tcl_registry::bpf_op::BpfDeclKind::Allow) => true,
            Some(tcl_registry::bpf_op::BpfDeclKind::Deny) => false,
            _ => continue,
        };
        if args.is_empty() {
            return Err(BpfError::new(
                BpfDiag::CapabilityDenied,
                *span,
                format!("`{cmd}` expects: {cmd} CMD ?CMD …?"),
            ));
        }
        for index in 0..args.len() {
            let verb = source.scalar(tokens.as_ref(), args, index).ok_or_else(|| {
                BpfError::new(
                    BpfDiag::CapabilityDenied,
                    *span,
                    "capability declarations require original static command names",
                )
            })?;
            if !is_governable(&verb, registry) {
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
            let descriptor = registry
                .get_for_surface(&verb, registry.own_surface_query())
                .expect("the governable descriptor was selected above")
                .descriptor_key();
            if kind {
                policy.allow.get_or_insert_with(Vec::new).push(descriptor);
            } else {
                policy.deny.push(descriptor);
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
    check_script(script, policy, registry)
}

fn check_script(
    script: &Script,
    policy: &CapabilityPolicy,
    registry: &CommandRegistry,
) -> Result<(), BpfError> {
    for stmt in &script.statements {
        match stmt {
            Statement::Call { span, .. } | Statement::Barrier { span, .. } => {
                let Some(spec) = selected_command(stmt, registry)? else {
                    continue;
                };
                let cmd = spec.name;
                if !policy.permits(
                    spec.descriptor_key(),
                    spec.bpf_op.is_some_and(|op| op.effects.is_gated()),
                ) {
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

#[cfg(test)]
mod original_operand_tests {
    use super::*;
    use tcl_compiler::ir::WordExpr;

    #[test]
    fn policy_selection_requires_the_actual_original_head_and_operand() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        for changed_head in [false, true] {
            let mut module = crate::source::lower_bpf_source(
                if changed_head {
                    "skip drop"
                } else {
                    "deny drop"
                },
                registry,
            );
            let source = OriginalBpfSource::from_module(&module);
            if !changed_head {
                let policy = collect_policy(&module.top_level, registry, &source).unwrap();
                let drop = registry
                    .get_for_surface("drop", registry.own_surface_query())
                    .unwrap();
                assert!(!policy.permits(drop.descriptor_key(), false));
            }
            let Statement::Call {
                command,
                args,
                tokens: Some(tokens),
                ..
            } = &mut module.top_level.statements[0]
            else {
                panic!("original capability command");
            };
            let index = if changed_head { 0 } else { 1 };
            let replacement = if changed_head { "deny" } else { "pass" };
            let WordExpr::Literal { text, .. } = &mut tokens.word_exprs[index] else {
                panic!("original literal operand");
            };
            *text = replacement.into();
            tokens.argv_texts[index] = replacement.into();
            if changed_head {
                *command = replacement.into();
            } else {
                args[0] = replacement.into();
            }
            assert_eq!(
                collect_policy(&module.top_level, registry, &source)
                    .unwrap_err()
                    .code,
                BpfDiag::CapabilityDenied
            );
        }
        let mut module = crate::source::lower_bpf_source("deny drop", registry);
        let source = OriginalBpfSource::from_module(&module);
        let Statement::Call { tokens, .. } = &mut module.top_level.statements[0] else {
            panic!("original capability command");
        };
        *tokens = None;
        assert_eq!(
            collect_policy(&module.top_level, registry, &source)
                .unwrap_err()
                .code,
            BpfDiag::CapabilityDenied
        );
    }
}
