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

//! The command table: registration and guards, command tokens and generations, publication, rename and removal, lookup and resolution, ensemble dispatch and the invocation state of a running native command.

use super::{
    CommandGenerations, CommandRenameTransaction, CommandSidecarHandle, CommandSlot,
    NativeHandlerMetadata, NativeRegisteredCommandToken, NativeUsageScope, Vm,
};
use crate::value::Value;
use tcl_core_types::NameBytes;
use tcl_runtime_api::Completion;

mod builtin_registration;
mod command_guards;
mod command_tokens;
mod ensemble_dispatch;
mod native_publication;
mod publication;
mod removal;
mod rename;
mod resolution;

/// A command lookup capability failure, independent of guest not-found errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeCommandLookupUnavailable {
    /// No independently selected native naming policy.
    ProtocolUnavailable,
    /// The actual namespace object/token is unavailable.
    NamespaceContextUnavailable,
    /// The selected slot has a native worker awaiting an owned registrar commit.
    PendingImplementationUnavailable(tcl_runtime_api::CommandSlot<NameBytes>),
    /// This interpreter's name world has retired.
    Retired,
}

impl std::fmt::Display for NativeCommandLookupUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ProtocolUnavailable => "native command name protocol is unavailable",
            Self::NamespaceContextUnavailable => "native command namespace context is unavailable",
            Self::PendingImplementationUnavailable(_) => {
                "selected native command implementation has not committed"
            }
            Self::Retired => "native command interpreter has retired",
        })
    }
}

impl std::error::Error for NativeCommandLookupUnavailable {}

impl NativeRegisteredCommandToken {
    /// The generation that must still occupy the captured exact slot.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Exact command address, without parsing its reporting spelling.
    #[must_use]
    pub fn slot(&self) -> &CommandSlot {
        &self.slot
    }
}

/// Identity domain for metadata that follows a command binding.  Tcl permits
/// a hidden token and a visible command to have the same spelling at the same
/// time; consequently a bare `String` is not a command identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CommandSidecarKey {
    Visible(String),
    Hidden(String),
}

impl CommandSidecarHandle {
    /// The binding this operation still belongs to.  A deletion detaches an
    /// in-flight handle before Tcl can create another binding at the same
    /// spelling, so it can never be retargeted to that later lifecycle.
    pub(crate) fn key(&self) -> Option<CommandSidecarKey> {
        self.0.borrow().clone()
    }

    /// Whether this in-flight operation still belongs to a command binding.
    /// Deletion clears the cell before Tcl callbacks run; trace iteration must
    /// re-check this after every re-entrant callback rather than continuing a
    /// cloned pre-mutation trace list.
    pub(crate) fn is_attached(&self) -> bool {
        self.0.borrow().is_some()
    }
}

impl CommandSidecarKey {
    pub(crate) fn visible(name: impl Into<String>) -> Self {
        Self::Visible(name.into())
    }

    pub(crate) fn hidden(name: impl Into<String>) -> Self {
        Self::Hidden(name.into())
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Visible(name) | Self::Hidden(name) => name,
        }
    }

    fn into_name(self) -> String {
        match self {
            Self::Visible(name) | Self::Hidden(name) => name,
        }
    }
}

impl CommandRenameTransaction {
    pub(crate) fn old_key(&self) -> &str {
        &self.old_key
    }

    pub(crate) fn new_key(&self) -> &str {
        &self.new_key
    }
}

pub(crate) enum CommandPublicationOutcome {
    Published(String),
    Unavailable,
}

impl CommandPublicationOutcome {
    pub(crate) fn into_key(self) -> Option<String> {
        match self {
            Self::Published(key) => Some(key),
            Self::Unavailable => None,
        }
    }
}

impl CommandGenerations {
    pub(super) fn insert(&mut self, key: String, generation: u64) -> Option<u64> {
        let previous = self.by_key.insert(key.clone(), generation);
        if let Some(previous) = previous {
            self.forget(previous, &key);
        }
        self.keys.entry(generation).or_default().push(key);
        previous
    }

    pub(super) fn remove(&mut self, key: &str) -> Option<u64> {
        let generation = self.by_key.remove(key)?;
        self.forget(generation, key);
        Some(generation)
    }

    pub(super) fn forget(&mut self, generation: u64, key: &str) {
        if let Some(keys) = self.keys.get_mut(&generation) {
            keys.retain(|indexed| indexed != key);
            if keys.is_empty() {
                self.keys.remove(&generation);
            }
        }
    }

    pub(super) fn get(&self, key: &str) -> Option<&u64> {
        self.by_key.get(key)
    }

    pub(super) fn contains_key(&self, key: &str) -> bool {
        self.by_key.contains_key(key)
    }

    fn key_of(&self, generation: u64) -> Option<&String> {
        self.keys.get(&generation)?.first()
    }

    fn is_bound(&self, generation: u64) -> bool {
        self.keys.contains_key(&generation)
    }
}

impl Vm {
    pub(crate) fn enter_native_usage_scope(
        &mut self,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> NativeUsageScope {
        NativeUsageScope {
            previous: std::mem::replace(
                &mut self.native_invocation.usage_rewrites,
                Self::borrow_native_argument_usage(usage),
            ),
            revision: self.native_invocation.usage_revision,
        }
    }

    pub(crate) fn leave_native_usage_scope(&mut self, scope: NativeUsageScope) {
        if scope.revision == self.native_invocation.usage_revision {
            self.native_invocation.usage_rewrites = scope.previous;
        }
    }

    pub(crate) fn take_native_handler_metadata(&mut self) -> NativeHandlerMetadata {
        NativeHandlerMetadata {
            name: self.native_invocation.name.take(),
            original_name: self.native_invocation.original_name.take(),
            builtin_identity: self.native_invocation.builtin_identity.take(),
            sidecar: self.native_invocation.sidecar.take(),
        }
    }

    pub(crate) fn restore_native_handler_metadata(&mut self, metadata: NativeHandlerMetadata) {
        self.native_invocation.name = metadata.name;
        self.native_invocation.original_name = metadata.original_name;
        self.native_invocation.builtin_identity = metadata.builtin_identity;
        self.native_invocation.sidecar = metadata.sidecar;
    }

    pub(crate) fn reset_native_ensemble_rewrite(
        &mut self,
        event: tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent,
    ) -> Result<(), Completion<Value>> {
        let logical = self.logical_providers.names.and_then(|provider| {
            (provider.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation)
                .then_some(tcl_native_model::native_ensemble_rewrite::LogicalEnsembleRewriteProvider::Tcl84CoreSimulation)
        });
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .ensemble_rewrite_protocol(logical)
            .or_else(|| {
                self.native_invocation_dialect()
                    .ensemble_rewrite_protocol(logical)
            })
        else {
            return Err(
                self.refuse_host_command("ensemble rewrite reset protocol is unavailable".into())
            );
        };
        if protocol.resets_at(event) {
            self.native_invocation.usage_rewrites.clear();
            self.native_invocation.usage_revision =
                self.native_invocation.usage_revision.wrapping_add(1);
        }
        Ok(())
    }

    /// Record the word a builtin is being invoked under (its source `objv[0]`).
    pub(crate) fn set_invoked_name(&mut self, name: &str) {
        self.native_invocation.builtin_identity = self
            .resolve_command_fqn(self.current_ns(), name)
            .and_then(|key| self.stock_native_identity(&key));
        self.native_invocation.name = Some(name.to_owned());
        self.native_invocation.original_name = Some(Value::string(name));
    }

    pub(crate) fn set_invoked_name_value(&mut self, original: &Value, selected_key: &str) {
        self.native_invocation.name = original
            .existing_string_representation()
            .map(|text| text.to_string());
        self.native_invocation.original_name = Some(
            if self.actual_native_string_protocol()
                == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            {
                // Jim's invocation metadata borrows argv[0]. The active
                // argument vector owns the native reference during dispatch.
                original.native_lifetime_lease().into_value()
            } else {
                original.clone()
            },
        );
        self.native_invocation.builtin_identity = self.stock_native_identity(selected_key);
    }

    pub(crate) fn invoked_name_value(&self) -> Option<Value> {
        self.native_invocation.original_name.clone()
    }

    /// Exact registered implementation of the selected native handler.
    pub(crate) fn invoked_builtin_identity(&self) -> Option<&str> {
        self.native_invocation.builtin_identity.as_deref()
    }

    pub(crate) fn retain_invoked_builtin_identity(&mut self, identity: Option<String>) {
        self.native_invocation.builtin_identity = identity;
    }

    /// The word the current builtin was invoked under, if recorded.
    pub(crate) fn invoked_name(&self) -> Option<&str> {
        self.native_invocation.name.as_deref()
    }
}
