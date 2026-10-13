// SPDX-License-Identifier: AGPL-3.0-or-later
//! C alias registration coordinates remain separate from movable command slots.

use super::{
    native_operation_currency::NativeOperationScope, Command, Interp, NativeAliasHeader, Rc,
};
use tcl_cmd_core::CmdError;
use tcl_syntax::naming::{
    NameProjectionUnavailable, NativeNameContext, NativeNameProjection, NativeNameProtocol,
    NativeNamePurpose,
};
use tcl_syntax::value::ValueError;

impl Interp {
    pub(super) fn select_original_alias_registration_order(&self) -> Result<(), ValueError> {
        let names = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "alias registration order issuer",
            ))?
            .recipe();
        let abi = tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(None).ok_or(
            ValueError::CommandProtocolUnavailable("alias registration table ABI"),
        )?;
        self.native_alias_registrations
            .borrow_mut()
            .select_order(names, abi);
        Ok(())
    }

    pub(crate) fn original_alias_registration_binding(
        &self,
        name: &[u8],
    ) -> Result<Option<(u64, Command)>, ValueError> {
        let recipe = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "alias registration query issuer",
            ))?
            .recipe();
        if recipe.is_jim084() {
            let world = self.namespaces();
            return Ok(world
                .resolve_generation(self.current_ns.get(), name)
                .zip(world.resolve(self.current_ns.get(), name)));
        }
        let selected = recipe.alias_registration_query_input(name).map_err(|_| {
            ValueError::CommandProtocolUnavailable("alias registration query coordinate")
        })?;
        let generation = self
            .native_alias_registrations
            .borrow()
            .query(&selected)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("alias registration query purpose")
            })?
            .copied();
        Ok(generation.and_then(|generation| {
            self.raw_command_by_generation(generation)
                .map(|command| (generation, command))
        }))
    }

    pub(crate) fn delete_original_alias_registration(
        &mut self,
        name: &[u8],
    ) -> Result<bool, ValueError> {
        let Some((generation, command)) = self.original_alias_registration_binding(name)? else {
            return Ok(false);
        };
        if !matches!(command, Command::Alias { .. } | Command::ParentAlias { .. }) {
            return Ok(false);
        }
        Ok(self.delete_command_generation(generation))
    }

    fn replace_original_alias_registration(
        &self,
        generation: u64,
        original: &Rc<NativeAliasHeader>,
        replacement: &Rc<NativeAliasHeader>,
        report: &[u8],
    ) -> bool {
        if self.namespaces_mut().replace_alias_registration(
            generation,
            original,
            replacement,
            report,
        ) {
            return true;
        }
        let mut hidden = self.hidden.borrow_mut();
        let Some(binding) = hidden
            .values_mut()
            .find(|binding| binding.generation == generation)
        else {
            return false;
        };
        let (registration_name, publication_name) = match &mut binding.command {
            Command::Alias {
                registration_name,
                publication_name,
                ..
            }
            | Command::ParentAlias {
                registration_name,
                publication_name,
                ..
            } => (registration_name, publication_name),
            _ => return false,
        };
        if !registration_name
            .as_ref()
            .is_some_and(|actual| Rc::ptr_eq(actual, original))
        {
            return false;
        }
        *registration_name = Some(Rc::clone(replacement));
        *publication_name = report.to_vec();
        true
    }

    pub(crate) fn complete_original_alias_registration(
        &mut self,
        name: &[u8],
        purpose: NativeNamePurpose,
        registration: &Rc<NativeAliasHeader>,
        generation: Option<u64>,
    ) -> Result<(), CmdError> {
        let recipe = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original alias registration issuer",
            ))?
            .recipe();
        let unavailable =
            |_| ValueError::CommandProtocolUnavailable("original alias registration coordinate");
        let selected = projection(recipe, purpose, name).map_err(unavailable)?;
        if selected
            .alias_registration_key()
            .map_err(unavailable)?
            .is_none()
        {
            return Ok(());
        }
        let Some(generation) = generation else {
            return Ok(());
        };
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
        self.select_original_alias_registration_order()?;
        if recipe != NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4) {
            let mut current = Rc::clone(registration);
            loop {
                let bytes = self.native_object_string_bytes(current.checked_ptr()?)?;
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(CmdError::from_execution_refusal)?;
                let selected = projection(recipe, purpose, &bytes).map_err(unavailable)?;
                if self
                    .native_alias_registrations
                    .borrow()
                    .query(&selected)
                    .map_err(unavailable)?
                    .is_none()
                {
                    if matches!(self.raw_command_by_generation(generation),
                        Some(Command::Alias { registration_name: Some(ref actual), .. }
                        | Command::ParentAlias { registration_name: Some(ref actual), .. }) if Rc::ptr_eq(actual, &current))
                    {
                        self.native_alias_registrations
                            .borrow_mut()
                            .install_unique(&selected, generation)
                            .map_err(unavailable)?;
                        self.set_result(current.checked_ptr()?);
                    }
                    return Ok(());
                }
                let token = recipe
                    .alias_registration_collision_token(&bytes)
                    .map_err(unavailable)?;
                let replacement = Rc::new(NativeAliasHeader::new(super::obj::Owned::fresh(
                    super::new_string(&token),
                )));
                if !self.replace_original_alias_registration(
                    generation,
                    &current,
                    &replacement,
                    &token,
                ) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original alias token replacement owner",
                    )
                    .into());
                }
                let retired = current.take_role();
                drop(retired);
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(CmdError::from_execution_refusal)?;
                current = replacement;
            }
        }
        loop {
            let prior = {
                let bytes = self.native_object_string_bytes(registration.checked_ptr()?)?;
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(CmdError::from_execution_refusal)?;
                let selected = projection(recipe, purpose, &bytes).map_err(unavailable)?;
                let prior = self
                    .native_alias_registrations
                    .borrow()
                    .prior(&selected)
                    .map_err(unavailable)?
                    .copied();
                let Some(prior) = prior else {
                    let current = self.raw_command_by_generation(generation);
                    if matches!(current, Some(Command::Alias { registration_name: Some(ref actual), .. }
                    | Command::ParentAlias { registration_name: Some(ref actual), .. })
                    if Rc::ptr_eq(actual, registration))
                    {
                        self.native_alias_registrations
                            .borrow_mut()
                            .install(&selected, generation)
                            .map_err(unavailable)?;
                    }
                    return Ok(());
                };
                prior
            };
            if self.raw_command_by_generation(prior).is_none() {
                if self.namespaces().original_command_deletion_started(prior) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original alias registration retirement remains in progress",
                    )
                    .into());
                }
                self.native_alias_registrations
                    .borrow_mut()
                    .forget_deleted(|actual| *actual == prior);
                continue;
            }
            self.retire_original_command_generation(prior, true)?;
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(CmdError::from_execution_refusal)?;
        }
    }
}

fn projection(
    recipe: NativeNameProtocol,
    purpose: NativeNamePurpose,
    bytes: &[u8],
) -> Result<NativeNameProjection<'_>, NameProjectionUnavailable> {
    match purpose {
        NativeNamePurpose::AliasPublication => {
            recipe.alias_publication_input(NativeNameContext::root(), bytes)
        }
        NativeNamePurpose::ChildAliasPublication => {
            recipe.child_alias_publication_input(NativeNameContext::root(), bytes)
        }
        _ => Err(NameProjectionUnavailable::PurposeNotModelled),
    }
}
