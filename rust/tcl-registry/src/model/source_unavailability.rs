// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Excluded command metadata remains separate from admitted source schemas.

use super::{ContextRegistry, ResolvedContext};
use crate::registry::NameProviders;
use tcl_dialect::model::{Family, InvocationRealm};

/// Why the actual assistance context excludes this Registry command.
/// These classifications provide no runtime command/table or entry proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSourceUnavailabilityKind {
    /// Related Registry metadata is unavailable under the complete context.
    Unavailable,
    /// The authored rule-loader surface excludes this literal command.
    RuleLoaderRefused,
}

/// Negative Registry availability metadata, independently of source identity.
/// The original command and its applicability must be supplied by another owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSourceUnavailability {
    command: String,
    kind: CommandSourceUnavailabilityKind,
    providers: NameProviders,
}
impl CommandSourceUnavailability {
    /// Canonical Registry descriptor name, never a selected native command.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// Exact excluded surface classification under the retained context.
    #[must_use]
    pub const fn kind(&self) -> CommandSourceUnavailabilityKind {
        self.kind
    }
    /// Every authored provider row, for advisory availability presentation.
    #[must_use]
    pub const fn providers(&self) -> &NameProviders {
        &self.providers
    }
}
impl ResolvedContext {
    /// Classify a Registry command excluded by the full context and realm.
    /// Unknown or unrelated names are not disabled metadata. An admitted row
    /// cannot issue this negative descriptor; packages and keyed floors remain
    /// the same owners used by ordinary source-schema selection.
    #[must_use]
    pub fn command_source_unavailability(
        &self,
        registry: &crate::CommandRegistry,
        command: &str,
        realm: InvocationRealm,
    ) -> Option<CommandSourceUnavailability> {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        if self
            .resolve_spec_in_realm(registry, command, realm)
            .is_some()
        {
            return None;
        }
        let providers = registry.source_availability_providers(command)?;
        if !self.is_related_to_a_provider_of(&providers) {
            return None;
        }
        let refused = self
            .environment
            .core
            .is_some_and(|core| core.family == Family::F5Irules)
            && crate::irules_policy::rule_loader_refuses(command, realm);
        Some(CommandSourceUnavailability {
            command: command.to_owned(),
            kind: if refused {
                CommandSourceUnavailabilityKind::RuleLoaderRefused
            } else {
                CommandSourceUnavailabilityKind::Unavailable
            },
            providers,
        })
    }
}
impl ContextRegistry {
    /// Negative source availability from this exact immutable generation.
    /// This never selects a handler or produces an admitted invocation schema.
    #[must_use]
    pub fn command_source_unavailability(
        &self,
        command: &str,
        realm: InvocationRealm,
    ) -> Option<CommandSourceUnavailability> {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        self.context()
            .command_source_unavailability(self.commands(), command, realm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excluded_command_metadata_uses_related_providers_and_actual_realm() {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        let old = crate::model::ingress::static_context_for("tcl8.4");
        let new = crate::model::ingress::static_context_for("tcl8.6");
        let loader = InvocationRealm::RuleLoader;
        assert_eq!(
            old.command_source_unavailability("dict", loader)
                .unwrap()
                .kind(),
            CommandSourceUnavailabilityKind::Unavailable
        );
        assert!(new.command_source_unavailability("dict", loader).is_none());
        assert!(
            old.command_source_unavailability("no_registered_command", loader)
                .is_none()
        );
        let jim = crate::model::ingress::static_context_for("jim");
        assert!(
            jim.command_source_unavailability("coroutine", loader)
                .is_some()
        );
        assert!(
            jim.command_source_unavailability("system", loader)
                .is_none()
        );
        let irules = crate::model::ingress::static_context_for("f5-irules");
        assert_eq!(
            irules
                .command_source_unavailability("rename", loader)
                .unwrap()
                .kind(),
            CommandSourceUnavailabilityKind::RuleLoaderRefused
        );
        assert!(
            irules
                .command_source_unavailability("rename", InvocationRealm::InterpreterRuntime)
                .is_none()
        );
    }
    #[test]
    fn excluded_overlay_rows_remain_in_the_actual_store_availability_query() {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        const LATER: &[tcl_dialect::model::SpecSurface] =
            &[tcl_dialect::model::SpecSurface::core_in(
                Family::Tcl,
                &[("8.6", None)],
            )];
        let mut store = crate::CommandRegistry::build_default();
        store.insert(crate::CommandSpec {
            name: "availability_overlay_probe",
            surface: Some(LATER),
            ..crate::CommandSpec::DEFAULT
        });
        let old = crate::model::ingress::static_context_for("tcl8.4");
        let new = crate::model::ingress::static_context_for("tcl8.6");
        assert!(
            store
                .command_names_in_any_dialect()
                .any(|name| name == "availability_overlay_probe")
        );
        let advice = old
            .context()
            .command_source_unavailability(
                &store,
                "availability_overlay_probe",
                InvocationRealm::RuleLoader,
            )
            .unwrap();
        assert_eq!(advice.command(), "availability_overlay_probe");
        assert_eq!(advice.providers().rows, LATER);
        assert!(
            new.context()
                .command_source_unavailability(
                    &store,
                    "availability_overlay_probe",
                    InvocationRealm::RuleLoader
                )
                .is_none()
        );
    }
}
