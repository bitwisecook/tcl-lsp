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
        if projection(recipe, purpose, name)
            .map_err(unavailable)?
            .alias_registration_replacement_key()
            .map_err(unavailable)?
            .is_none()
        {
            return Ok(());
        }
        let Some(generation) = generation else {
            return Ok(());
        };
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
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
