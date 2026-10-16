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

/// Actual caller namespace/frame and each subsequently selected namespace birth.
/// Token observations add no Native reference or lookup purpose.
pub(crate) struct OriginalNamespaceConsumer {
    interpreter: Interp,
    namespace: NsId,
    frame: super::native_arguments::OriginalArgumentFrame,
    tokens: std::cell::RefCell<
        Vec<(
            tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken,
            tcl_syntax::native_namespace_name::NativeNamespaceLifecycle,
        )>,
    >,
}
impl OriginalNamespaceConsumer {
    pub(crate) fn check(
        &self,
        arguments: &super::native_arguments::OriginalArguments,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        arguments.check_for(&self.interpreter)?;
        self.frame.check(&self.interpreter, arguments)?;
        if self.interpreter.current_ns() != self.namespace
            || self.tokens.borrow().iter().any(|(token, lifecycle)| {
                token.lifecycle() != *lifecycle
                    || !self
                        .interpreter
                        .namespaces()
                        .owns_namespace_name_token(token)
            })
        {
            return Err(arguments.retain_refusal(
                ValueError::CommandProtocolUnavailable("original namespace consumer incarnation")
                    .into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn observe(
        &self,
        namespace: NsId,
        arguments: &super::native_arguments::OriginalArguments,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.check(arguments)?;
        let token = self
            .interpreter
            .native_namespace_token_at(namespace)
            .ok_or_else(|| {
                arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable("original namespace consumer target")
                        .into(),
                )
            })?;
        let lifecycle = token.lifecycle();
        self.tokens.borrow_mut().push((token, lifecycle));
        self.check(arguments)
    }
}

impl Interp {
    pub(crate) fn original_namespace_consumer(
        &self,
        arguments: &super::native_arguments::OriginalArguments,
    ) -> Result<OriginalNamespaceConsumer, tcl_cmd_core::CmdError> {
        arguments.check_for(self)?;
        let owner = OriginalNamespaceConsumer {
            interpreter: self.clone(),
            namespace: self.current_ns(),
            frame: super::native_arguments::OriginalArgumentFrame::capture(self),
            tokens: std::cell::RefCell::new(Vec::new()),
        };
        owner.observe(owner.namespace, arguments)?;
        Ok(owner)
    }

    /// Observe the actual holder through the existing namespace ledger.
    pub(crate) fn native_namespace_token_at(
        &self,
        namespace: NsId,
    ) -> Option<tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken> {
        self.namespaces()
            .namespace_name_token(self.native_command_interpreter, namespace)
    }

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
        self.native_namespace_object_lookup_with_checks(self.current_ns(), original, None, None)
    }

    pub(super) fn native_namespace_object_lookup_using<E>(
        &mut self,
        original: *mut TclObj,
        check: &(impl Fn() -> Result<(), E> + ?Sized),
        refuse: &impl Fn(ValueError) -> E,
    ) -> Result<Option<NsId>, E> {
        crate::list::with_native_conversion_check(check, refuse, |stage, retain| {
            self.native_namespace_object_lookup_with_checks(
                self.current_ns(),
                original,
                Some(stage),
                Some(retain),
            )
        })
    }

    /// Select an existing original-object lookup in the actual caller namespace.
    pub(crate) fn native_namespace_object_lookup_at_using<E>(
        &mut self,
        namespace: NsId,
        original: *mut TclObj,
        check: &(impl Fn() -> Result<(), E> + ?Sized),
        refuse: &impl Fn(ValueError) -> E,
    ) -> Result<Option<NsId>, E> {
        crate::list::with_native_conversion_check(check, refuse, |stage, retain| {
            self.native_namespace_object_lookup_with_checks(
                namespace,
                original,
                Some(stage),
                Some(retain),
            )
        })
    }

    fn native_namespace_object_lookup_with_checks(
        &mut self,
        current: NsId,
        original: *mut TclObj,
        stage: Option<&dyn Fn() -> Result<(), ValueError>>,
        retain: Option<&dyn Fn(ValueError) -> ValueError>,
    ) -> Result<Option<NsId>, ValueError> {
        let check = || {
            if let Some(check) = stage {
                check()?;
            }
            obj::check_native_liveness(original)
        };
        let bytes_for = |actual: &mut Interp| {
            if stage.is_some() {
                actual.native_object_string_bytes_using(original, &check, &|error| match retain {
                    Some(retain) => retain(error),
                    None => error,
                })
            } else {
                actual.native_string_bytes(&original)
            }
        };
        if stage.is_some() {
            check()?;
        }

        let dialect = self.native_invocation_dialect();
        let Some(protocol) = dialect.native_namespace_name_protocol() else {
            if obj::native_namespace_name::cache(original).is_some() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native namespace-name object issuer",
                ));
            }
            let bytes = bytes_for(self)?;
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
            Some(bytes_for(self)?)
        } else {
            None
        };
        if stage.is_some() {
            check()?;
        }
        let reference_ns = if bytes
            .as_ref()
            .is_some_and(|bytes| tcl_syntax::naming::written_command_parts(&bytes).absolute)
        {
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
        let reference_guard = stage.is_some().then(|| reference.clone());
        let reference_current = |actual: &Interp| {
            check()?;
            if reference_guard
                .as_ref()
                .is_some_and(|reference| !actual.namespaces().owns_namespace_name_token(reference))
            {
                return Err(match retain {
                    Some(retain) => retain(ValueError::CommandProtocolUnavailable(
                        "changed original namespace reference incarnation",
                    )),
                    None => ValueError::CommandProtocolUnavailable(
                        "changed original namespace reference incarnation",
                    ),
                });
            }
            Ok(())
        };
        if stage.is_some() {
            reference_current(self)?;
        }
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
                let current = || reference_current(self);
                obj::native_namespace_name::retire_using(
                    original,
                    stage.map(|_| &current as &dyn Fn() -> Result<(), ValueError>),
                )?;
            }
        }
        if bytes.is_none() {
            bytes = Some(bytes_for(self)?);
        }
        if stage.is_some() {
            reference_current(self)?;
        }
        let bytes = bytes.expect("native namespace string was materialised");
        let context = tcl_runtime_api::NsId(u32::try_from(reference_ns).map_err(|_| {
            ValueError::CommandProtocolUnavailable("native namespace context width")
        })?);
        let selected = match self.namespace_address_components(context.0 as usize, &bytes) {
            Ok((anchor, components)) => self
                .namespaces()
                .namespace_token_at_components(anchor, &components),
            Err(super::namespace_address::NamespaceAddressError::Projection {
                reason: tcl_syntax::naming::NameProjectionUnavailable::EmptyNamespaceOutsideRoot,
                ..
            }) => None,
            Err(error) => return Err(error.into_value_error()),
        };
        if stage.is_some() {
            reference_current(self)?;
        }
        let selected = selected
            .map(|namespace| {
                self.namespaces()
                    .namespace_name_token(interpreter, namespace)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "native namespace target incarnation",
                    ))
                    .map(|target| (namespace, target))
            })
            .transpose()?
            .filter(|(_, target)| recipe.permits_target(target.lifecycle()));
        let namespace = selected.as_ref().map(|(namespace, _)| *namespace);
        if let Some((_, target)) = selected {
            let reference = (!tcl_syntax::naming::written_command_parts(&bytes).absolute
                || recipe.absolute_references_global())
            .then_some(reference);
            let current = || reference_current(self);
            obj::native_namespace_name::install_using(
                original,
                Cache::resolved(recipe.version(), target, reference),
                protocol,
                stage.map(|_| &current as &dyn Fn() -> Result<(), ValueError>),
                retain,
            )?;
        } else if recipe.missing_installs_unresolved() {
            let current = || reference_current(self);
            obj::native_namespace_name::install_using(
                original,
                Cache::unresolved(interpreter),
                protocol,
                stage.map(|_| &current as &dyn Fn() -> Result<(), ValueError>),
                retain,
            )?;
        }
        Ok(namespace)
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
                let bytes = self.materialize_jim_namespace(self.current_ns())?;
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
