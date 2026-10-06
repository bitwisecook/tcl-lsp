// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original rename operands and their selected command-table extents.

use super::Interp;
use crate::obj::TclObj;
use tcl_syntax::{naming::NativeNameContext, value::ValueError};

impl Interp {
    /// Materialize the original operands before selecting rename's native name
    /// purposes. Jim's table owner applies its retained namespace qualification;
    /// C passes its selected CString bytes to the mutation owner.
    pub(crate) fn original_rename_operands(
        &mut self,
        old: *mut TclObj,
        new: *mut TclObj,
    ) -> Result<(Vec<u8>, Vec<u8>), ValueError> {
        let old = self.native_object_string_bytes(old)?;
        let new = self.native_object_string_bytes(new)?;
        let recipe = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original rename name purposes",
            ))?
            .recipe();
        let namespaces = self.namespaces.borrow();
        let current = self.current_ns();
        let path = namespaces.native_context_path(current).ok_or(
            ValueError::CommandProtocolUnavailable("original rename namespace"),
        )?;
        let context = if recipe.is_jim084() {
            NativeNameContext::with_jim_namespace(
                &path,
                namespaces.jim_namespace_bytes(current).ok_or(
                    ValueError::CommandProtocolUnavailable("original rename Jim namespace"),
                )?,
            )
        } else {
            NativeNameContext::new(&path)
        };
        let selected_old = recipe.rename_source_input(context, &old).map_err(|_| {
            ValueError::CommandProtocolUnavailable("original rename source purpose")
        })?;
        let selected_new = recipe
            .rename_destination_input(context, &new)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original rename destination purpose")
            })?;
        if recipe.is_jim084() {
            Ok((
                selected_old.original().to_vec(),
                selected_new.original().to_vec(),
            ))
        } else {
            Ok((
                selected_old.selected().to_vec(),
                selected_new.selected().to_vec(),
            ))
        }
    }
    /// Use the actual restored source token and selected destination slot for
    /// the native alias-loop error. Neither diagnostic display is re-parsed.
    pub(crate) fn original_rename_alias_loop_name(
        &self,
        old: &[u8],
        new: &[u8],
    ) -> Result<Vec<u8>, ValueError> {
        let unavailable =
            || ValueError::CommandProtocolUnavailable("alias rename diagnostic binding");
        let recipe = self
            .native_invocation_dialect()
            .native_name_protocol()
            .ok_or_else(unavailable)?;
        let namespaces = self.namespaces.borrow();
        let current = self.current_ns();
        let generation = namespaces
            .resolve_generation(current, old)
            .ok_or_else(unavailable)?;
        let source = namespaces
            .native_command_simple_at_node(generation)
            .ok_or_else(unavailable)?;
        let path = namespaces
            .native_context_path(current)
            .ok_or_else(unavailable)?;
        let destination = recipe
            .rename_destination_slot(NativeNameContext::new(&path), new)
            .map_err(|_| unavailable())?;
        recipe
            .rename_alias_loop_name(source, destination.simple.as_bytes())
            .map(<[u8]>::to_vec)
            .ok_or_else(unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::Code;
    use std::rc::Rc;

    #[test]
    fn alias_rename_diagnostic_matches_all_five_original_native_bindings() {
        for version in tcl_dialect::TclVersion::ALL {
            let profile =
                tcl_registry::model::resolve_environment(version.dialect_name()).unit_profile();
            let mut interp = Interp::with_native_core(
                Rc::new(tcl_host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(interp.eval_str(b"interp alias {} a {} b; set code [catch {rename a b} message]; list $code $message [info commands a] [info commands b]"), Code::Ok);
            let name = if version == tcl_dialect::TclVersion::V8_4 {
                "a"
            } else {
                "b"
            };
            assert_eq!(
                interp.result_bytes(),
                format!(
                    "1 {{cannot define or rename alias \"{name}\": would create a loop}} a {{}}"
                )
                .as_bytes(),
                "{version:?}"
            );
            assert!(matches!(
                interp.original_rename_alias_loop_name(b"missing", b"b"),
                Err(ValueError::CommandProtocolUnavailable(_))
            ));
            assert_eq!(interp.eval_str(b"rename a saved; proc a {} {return REPLACEMENT}; list [a] [info commands saved]"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"REPLACEMENT saved");
        }
    }
}
