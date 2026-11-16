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

//! Constructing an interpreter and selecting the world it runs in: host, release, pinned context, invocation protocols and the bootstrap.

use super::{Interp, InvocationProtocols};
use crate::frame::VarError;
use crate::obj::TclObj;
use std::rc::Rc;

mod bootstrap;
mod construction;
mod pinned_context;

#[cfg(test)]
pub(crate) use construction::default_host;

impl Interp {
    /// Actual Jim interpreter objects; source grammar cannot issue this context.
    pub(crate) fn native_jim_object_context(
        &self,
    ) -> Result<Rc<crate::native_source::NativeJimObjectContext>, tcl_syntax::value::ValueError>
    {
        self.native_jim_object_context_in(self.native_invocation_dialect())
    }

    pub(crate) fn associate_native_jim_arguments(
        &self,
        values: &[*mut TclObj],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.invocation_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Ok(());
        }
        let context = self.native_jim_object_context()?;
        for &value in values {
            crate::native_source::bind_context(value, &context)?;
        }
        Ok(())
    }

    pub(super) fn associate_native_jim_variable_value(
        &self,
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        self.associate_native_jim_arguments(&[value])
            .map_err(|error| {
                self.clone().refuse_native_access(
                    error
                        .native_access_refusal()
                        .expect("Jim context association is a host refusal"),
                );
                VarError::NameProtocolUnavailable
            })
    }

    pub(crate) fn native_jim_object_context_in(
        &self,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Rc<crate::native_source::NativeJimObjectContext>, tcl_syntax::value::ValueError>
    {
        if dialect.native_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original source context",
            ));
        }
        if let Some(context) = self.jim_object_context.borrow().as_ref() {
            if !context.is_live() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "retired Jim interpreter",
                ));
            }
            context.select_numeric_host(self.host());
            return Ok(Rc::clone(context));
        }
        let context = crate::native_source::NativeJimObjectContext::new(dialect)?;
        context.select_numeric_host(self.host());
        self.namespaces
            .borrow_mut()
            .adopt_jim_root_namespace(context.empty_object().clone());
        let namespace = self
            .namespaces
            .borrow_mut()
            .take_jim_namespace_owner(crate::namespace::GLOBAL)
            .expect("authentic Jim root namespace reference");
        self.frames
            .borrow_mut()
            .retain_jim_root_namespace(namespace);
        *self.jim_object_context.borrow_mut() = Some(Rc::clone(&context));
        Ok(context)
    }

    pub(super) fn configure_variable_storage(&self) {
        let dialect = self.native_invocation_dialect();
        if let Some(names) = dialect.native_name_protocol() {
            let policy = names.variable_lookup_policy();
            let mut frames = self.frames.borrow_mut();
            let mut namespaces = self.namespaces.borrow_mut();
            frames.variable_lookup_policy = policy;
            namespaces.variable_lookup_policy = policy;
            frames.variable_container_model = policy.container_model();
            namespaces.variable_container_model = policy.container_model();
            namespaces.variable_link_binding = policy.link_binding();
        }
        // Missing naming authority cannot replace an existing physical model.
        let names = self.name_policy_protocol();
        let recipe = self
            .selected_variable_table_protocol()
            .map(|protocol| protocol.recipe());
        let mut frames = self.frames.borrow_mut();
        let mut namespaces = self.namespaces.borrow_mut();
        frames.variable_hash_recipe = recipe;
        namespaces.variable_hash_recipe = recipe;
        frames.variable_string_protocol = dialect.native_string_protocol();
        namespaces.variable_string_protocol = dialect.native_string_protocol();
        namespaces.variable_name_protocol = names.map(|names| names.recipe());
    }

    pub(super) fn variable_storage_matches(
        &self,
        policy: tcl_dialect::VariableLookupPolicy,
    ) -> bool {
        let frames = self.frames.borrow();
        let namespaces = self.namespaces.borrow();
        frames.variable_lookup_policy == policy
            && namespaces.variable_lookup_policy == policy
            && frames.variable_container_model == policy.container_model()
            && namespaces.variable_container_model == policy.container_model()
            && namespaces.variable_link_binding == policy.link_binding()
    }

    pub(crate) fn selected_variable_table_protocol(
        &self,
    ) -> Option<tcl_registry::native_variable_table::NativeVariableTableProtocol> {
        let abi = tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(Some(0))?;
        let policy = self.name_policy_protocol()?;
        let dialect = self.native_invocation_dialect();
        match policy.authority() {
            tcl_syntax::naming::NamePolicyAuthority::Native => {
                dialect.native_variable_table_protocol(abi)
            }
            tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation => {
                dialect.authored_variable_table_protocol(policy, abi)
            }
        }
    }

    pub(crate) fn native_invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        if let Some(dialect) = self.0.invocation_dialect.get() {
            return dialect;
        }
        let dialect = tcl_registry::InvocationDialect::of_profile(self.dialect_profile());
        let dialect = if dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            && dialect.tcl_version.is_none()
            && dialect.core_point.is_none()
        {
            tcl_registry::InvocationDialect::for_version(self.runtime_version())
        } else {
            dialect
        };
        self.0.invocation_dialect.set(Some(dialect));
        dialect
    }

    pub(super) fn invocation_protocols(&self) -> InvocationProtocols {
        if let Some(protocols) = self.0.invocation_protocols.get() {
            return protocols;
        }
        let dialect = self.native_invocation_dialect();
        let name = dialect.native_name_protocol();
        let protocols = InvocationProtocols {
            string: dialect.native_string_protocol(),
            name,
            name_policy: name.and_then(|recipe| {
                let protocol = tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                    dialect.execution_point()?,
                )?;
                (protocol.recipe() == recipe).then_some(protocol)
            }),
        };
        self.0.invocation_protocols.set(Some(protocols));
        protocols
    }

    pub(crate) fn invocation_string_protocol(
        &self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        self.invocation_protocols().string
    }

    pub(crate) fn invocation_name_protocol(
        &self,
    ) -> Option<tcl_syntax::naming::NativeNameProtocol> {
        self.invocation_protocols().name
    }

    /// The lexer configuration scripts evaluate under: the pinned profile's
    /// grammar — `{*}` expansion off and the first-close `${…}` rule on when
    /// the interpreter emulates Tcl 8.4.
    pub(crate) fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::from_grammar(self.dialect_profile().grammar)
    }
}
