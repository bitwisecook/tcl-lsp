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

//! The profiles and native protocols of the current activation: source and execution profiles, string, name and compiled-variable protocols, and byte-array results.

use super::super::{ActualNativeProtocols, InterpState};
use crate::value::Value;
use std::rc::Rc;

impl InterpState {
    /// Source grammar and compiler target of the current explicit activation.
    /// The configured user dialect remains independent of the physical engine.
    #[must_use]
    pub(crate) fn source_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.active_native_profile.unwrap_or(self.pin.profile)
    }

    /// Invocation policy selected by a native token. Host aliases override only
    /// the explicit host activation; ordinary user activations retain their dialect.
    pub(crate) fn native_execution_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.execution_profile_for(self.active_native_profile.unwrap_or(self.pin.profile))
    }

    /// The pinned logical invocation profile, ignoring a scoped native host
    /// activation.
    pub(crate) fn logical_execution_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.execution_profile_for(self.pin.profile)
    }

    fn execution_profile_for(
        &self,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> &'static tcl_dialect::DialectProfile {
        if profile.is_fallback() {
            crate::environment::profile_for_dialect(self.runtime_version.dialect_profile_name())
        } else {
            profile
        }
    }

    pub(crate) fn native_invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        tcl_registry::InvocationDialect::of_static_profile(self.native_execution_profile())
    }

    /// Physical engine provenance at bootstrap and compilation ingress. Logical
    /// invocation policies continue to use `native_execution_profile` separately.
    pub(crate) fn actual_native_execution_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.active_native_profile
            .or(self.actual_engine_profile)
            .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl)
    }

    pub(crate) fn actual_native_invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        tcl_registry::InvocationDialect::of_static_profile(self.actual_native_execution_profile())
    }

    /// Advanced by every mutable borrow of the name world.
    pub(crate) fn name_world_generation(&self) -> u64 {
        self.name_world.generation()
    }

    pub(in super::super) fn actual_native_protocols(&self) -> ActualNativeProtocols {
        let profile = self.actual_native_execution_profile();
        let key = std::ptr::from_ref(profile).addr();
        if let Some((seen, protocols)) = self.actual_protocols.get()
            && seen == key
        {
            return protocols;
        }
        let actual = tcl_registry::InvocationDialect::of_static_profile(profile);
        let name = actual.native_name_protocol();
        let protocols = ActualNativeProtocols {
            string: actual.native_string_protocol(),
            name,
            name_policy: name.and_then(|recipe| {
                let policy = tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                    actual.execution_point()?,
                )?;
                (policy.recipe() == recipe).then_some(policy)
            }),
            error_stack: actual.error_stack_protocol(),
        };
        self.actual_protocols.set(Some((key, protocols)));
        protocols
    }

    pub(crate) fn actual_native_string_protocol(
        &self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        self.actual_native_protocols().string
    }

    pub(crate) fn actual_native_name_protocol(
        &self,
    ) -> Option<tcl_syntax::naming::NativeNameProtocol> {
        self.actual_native_protocols().name
    }

    /// Actual physical native provenance for scalar cache and object-carrier adapters.
    /// Logical source and event simulation policies do not authenticate this carrier.
    #[must_use]
    pub fn native_scalar_carrier_dialect(&self) -> tcl_registry::InvocationDialect {
        self.actual_native_invocation_dialect()
    }

    /// Produce binary backing for the current invocation's explicit purpose.
    /// `None` selects only the actual physical native engine. A supplied logical
    /// provider requires the current authored F5 source ingress and retains its
    /// sealed authored origin even when a different physical engine is present.
    /// Unsupported logical purposes never fall back to native creation.
    ///
    /// # Errors
    /// Refuses an unavailable producer without allocating or donating authority.
    pub fn byte_array_result(
        &self,
        bytes: Rc<[u8]>,
        provider: Option<tcl_registry::native_string_materialization::LogicalStringProvider>,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        let recipe = match provider {
            Some(provider) => tcl_registry::InvocationDialect::of_profile(self.source_profile())
                .byte_array_string_recipe(Some(provider))
                .filter(|recipe| recipe.logical_provider() == Some(provider)),
            None => self
                .actual_native_invocation_dialect()
                .byte_array_string_recipe(None),
        }
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "byte-array result producer",
        ))?;
        Ok(Value::from_byte_array_recipe(bytes, recipe))
    }

    /// Select native name semantics from the actual engine or an explicit
    /// authored logical provider. A compatible source version contributes no
    /// native authority; publication-time slots retain their original identity.
    #[must_use]
    pub fn name_policy_protocol(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        if let Some(provider) = self.logical_providers.names
            && let Some(selected) = self
                .native_invocation_dialect()
                .authored_logical_name_simulation(provider)
        {
            return Some(selected);
        }
        self.actual_native_protocols().name_policy
    }

    /// Select compiler-local semantics independently of native name lookup.
    /// F5 contexts require their explicit authored provider; the physical host
    /// compiler cannot fill a missing logical policy. Active native host calls
    /// select the actual compiler independently.
    #[must_use]
    pub fn compiled_variable_protocol(
        &self,
    ) -> Option<tcl_syntax::naming::NativeCompiledVariableProtocol> {
        let invocation = self.native_invocation_dialect();
        if matches!(
            invocation.family(),
            Some(tcl_dialect::model::Family::F5Tcl | tcl_dialect::model::Family::F5Irules)
        ) {
            return invocation
                .compiled_variable_protocol(self.logical_providers.compiled_variables);
        }
        self.actual_native_invocation_dialect()
            .native_compiled_variable_protocol()
    }

    /// Obtain complete native string operand bytes before selecting a name
    /// purpose. A byte array's resident string wins; a pure byte array uses and
    /// retains its selected materialisation recipe on the original value.
    ///
    /// # Errors
    /// Returns a host recipe refusal without creating a Tcl guest diagnostic.
    pub fn native_name_operand_bytes(
        &self,
        value: &Value,
    ) -> Result<Rc<[u8]>, tcl_syntax::native_string::NativeStringUnavailable> {
        if !value.native_object_is_live() {
            return Err(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater);
        }
        if let Some(bytes) = value.resident_string_bytes() {
            return Ok(bytes);
        }
        let protocol = self
            .actual_native_string_protocol()
            .ok_or(tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable)?;
        value.native_string_bytes_with_integer_formatter(
            protocol,
            self.host.native_integer_formatter(),
        )
    }

    pub(crate) fn expression_quote_control(
        &self,
    ) -> Option<tcl_registry::invocation_words::ExpressionQuoteControl> {
        let dialect = self.native_invocation_dialect();
        dialect.expression_quote_control().or_else(|| {
            self.logical_providers
                .quote
                .and_then(|provider| dialect.logical_expression_quote_control(provider))
        })
    }
}
