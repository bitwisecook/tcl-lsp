// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace objects resolve through the actual namespace owner.

use super::Interp;
use crate::{
    namespace::{NsId, GLOBAL},
    obj::{self, TclObj},
};
use tcl_runtime_api::{native_namespace_name::NativeNamespaceNameCache as Cache, Namespaces};
use tcl_syntax::{
    native_namespace_name::{
        NativeNamespaceCurrentPrimary as Primary, NativeNamespaceObjectProducer as Producer,
    },
    value::{ValueError, ValueOps},
};

#[cfg(test)]
mod tests;

impl Interp {
    /// Read actual arena lifetime through the existing namespace descriptor.
    pub(crate) fn native_current_namespace_lifecycle(
        &self,
    ) -> Result<tcl_syntax::native_namespace_name::NativeNamespaceLifecycle, ValueError> {
        self.namespaces()
            .namespace_name_token(self.native_command_interpreter, self.current_ns())
            .map(|token| token.lifecycle())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native current namespace incarnation",
            ))
    }

    /// Resolve an original object before reporting or reconstructing its name.
    /// Cache ownership is checked against the actual closed arena incarnation.
    pub(crate) fn native_namespace_object_lookup(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<NsId>, ValueError> {
        let current = self.current_ns();
        let dialect = self.native_invocation_dialect();
        let Some(protocol) = dialect.native_namespace_name_protocol() else {
            if obj::native_namespace_name::cache(original).is_some() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native namespace-name object issuer",
                ));
            }
            let bytes = self.native_string_bytes(&original)?;
            let current =
                tcl_runtime_api::NsId(u32::try_from(current).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("namespace context width")
                })?);
            return Ok(
                Namespaces::find_namespace_bytes_checked(self, current, &bytes)?
                    .map(|ns| ns.0 as usize),
            );
        };
        let recipe = protocol.recipe();
        let mut bytes = if recipe.absolute_references_global() {
            Some(self.native_string_bytes(&original)?)
        } else {
            None
        };
        let reference_ns = if bytes.as_ref().is_some_and(|bytes| bytes.starts_with(b"::")) {
            GLOBAL
        } else {
            current
        };
        let reference = self
            .namespaces()
            .namespace_name_token(self.native_command_interpreter, reference_ns)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native namespace reference incarnation",
            ))?;
        let interpreter = reference.interpreter();
        if let Some(cache) = obj::native_namespace_name::cache(original) {
            if cache.version() != recipe.version() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native namespace-name descriptor origin",
                ));
            }
            if cache.is_current(recipe, interpreter, &reference) {
                if let Some(target) = cache.namespace() {
                    if self.namespaces().owns_namespace_name_token(target) {
                        return Ok(Some(usize::try_from(target.token()).map_err(|_| {
                            ValueError::CommandProtocolUnavailable("native namespace token width")
                        })?));
                    }
                }
            }
            if !recipe.missing_installs_unresolved() {
                obj::native_namespace_name::retire(original)?;
            }
        }
        if bytes.is_none() {
            bytes = Some(self.native_string_bytes(&original)?);
        }
        let bytes = bytes.expect("native namespace string was materialised");
        let context = tcl_runtime_api::NsId(u32::try_from(reference_ns).map_err(|_| {
            ValueError::CommandProtocolUnavailable("native namespace context width")
        })?);
        let selected = Namespaces::find_namespace_bytes_checked(self, context, &bytes)?
            .map(|ns| ns.0 as usize);
        if let Some(namespace) = selected {
            let target = self
                .namespaces()
                .namespace_name_token(interpreter, namespace)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native namespace target incarnation",
                ))?;
            let reference = (!bytes.starts_with(b"::") || recipe.absolute_references_global())
                .then_some(reference);
            obj::native_namespace_name::install(
                original,
                Cache::resolved(recipe.version(), target, reference),
                protocol,
            )?;
        } else if recipe.missing_installs_unresolved() {
            obj::native_namespace_name::install(
                original,
                Cache::unresolved(interpreter),
                protocol,
            )?;
        }
        Ok(selected)
    }
}

impl tcl_cmd_core::namespace::NamespaceObjectBackend for Interp {
    fn current_namespace_object(&mut self) -> Result<*mut TclObj, ValueError> {
        let current = tcl_runtime_api::NsId(u32::try_from(self.current_ns()).map_err(|_| {
            ValueError::CommandProtocolUnavailable("native namespace context width")
        })?);
        self.produce_namespace_object(current, Producer::Current)
    }

    fn produce_namespace_object(
        &mut self,
        namespace: tcl_runtime_api::NsId,
        producer: Producer,
    ) -> Result<*mut TclObj, ValueError> {
        let dialect = self.native_invocation_dialect();
        let Some(protocol) = dialect.native_namespace_name_protocol() else {
            let Some(policy) = self.name_policy_protocol() else {
                return Err(ValueError::CommandProtocolUnavailable(
                    "namespace result producer name issuer",
                ));
            };
            if policy.recipe().is_jim084() {
                // The result and canonical holder are distinct actual objects.
                // Establish the real top-frame holder before reading its name.
                drop(self.jim_current_namespace_object()?);
            }
            let name = if let Some(recipe) = dialect.native_jim_lookup_protocol() {
                if namespace.0 as usize != self.current_ns() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "Jim original namespace result frame",
                    ));
                }
                let original = self.jim_current_namespace_object()?;
                let bytes =
                    tcl_syntax::value::ValueOps::native_string_bytes(self, &original.as_ptr())?;
                recipe.namespace_current_result(&bytes)
            } else {
                Namespaces::name_bytes(self, namespace)
            };
            let value = obj::new_string_bytes(&name);
            if policy.recipe().is_jim084() {
                obj::retain_jim_string_representation(value);
            } else if let Some(recipe) = dialect.authored_namespace_result_recipe(policy) {
                match recipe.result_primary(
                    producer,
                    tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Live,
                ) {
                    Some(Primary::String) => obj::native_namespace_name::current_string_84(value),
                    Some(Primary::UntypedString) => {}
                    Some(Primary::NamespaceName) | None => {
                        drop(obj::Owned::fresh(value));
                        return Err(ValueError::CommandProtocolUnavailable(
                            "authored namespace result producer",
                        ));
                    }
                }
            } else {
                drop(obj::Owned::fresh(value));
                return Err(ValueError::CommandProtocolUnavailable(
                    "native namespace result producer issuer",
                ));
            }
            return Ok(value);
        };
        let token = self
            .namespaces()
            .namespace_name_token(self.native_command_interpreter, namespace.0 as usize)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native current namespace incarnation",
            ))?;
        let primary = protocol
            .recipe()
            .result_primary(producer, token.lifecycle())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native current namespace lifetime",
            ))?;
        let value = obj::new_string_bytes(token.full_name().as_bytes());
        match primary {
            Primary::String => obj::native_namespace_name::current_string_84(value),
            Primary::UntypedString => {}
            Primary::NamespaceName => {
                if let Err(error) = obj::native_namespace_name::install(
                    value,
                    Cache::resolved(protocol.recipe().version(), token, None),
                    protocol,
                ) {
                    drop(obj::Owned::fresh(value));
                    return Err(error);
                }
            }
        }
        Ok(value)
    }
}

impl tcl_cmd_core::namespace::NamespaceDeleteBackend for Interp {
    fn delete_selected_namespace(
        &mut self,
        namespace: tcl_runtime_api::NsId,
    ) -> Result<(), ValueError> {
        self.delete_namespace_by_id(namespace.0 as usize);
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(())
    }
}
