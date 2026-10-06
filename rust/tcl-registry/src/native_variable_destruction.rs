// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected native variable destruction protocol, independent of source names.

pub use tcl_runtime_api::variable_destruction::{
    VariableDestructionPhase, VariableDestructionProtocol,
};

impl crate::InvocationDialect {
    /// Select teardown for the actual receiver kind and variable container model.
    /// Missing native policy supplies no callback or storage-lifetime proof.
    #[must_use]
    pub fn variable_destruction_protocol(
        self,
        array_root: bool,
    ) -> Option<VariableDestructionProtocol> {
        use tcl_dialect::{VariableContainerModel, model::Family};
        match (self.native_family, self.variable_container_model) {
            (Some(Family::Tcl), Some(VariableContainerModel::DistinctArray))
                if self.tcl_version.is_some() =>
            {
                Some(if array_root {
                    VariableDestructionProtocol::ArrayLookupThenRootCallbacksThenMembers
                } else {
                    VariableDestructionProtocol::CellContentsThenCallbacks
                })
            }
            (Some(Family::Jim), Some(VariableContainerModel::DictionaryValue)) => {
                Some(VariableDestructionProtocol::CellContentsThenCallbacks)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_storage_and_native_axes_are_required_independently() {
        let c = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(
            c.variable_destruction_protocol(true),
            Some(VariableDestructionProtocol::ArrayLookupThenRootCallbacksThenMembers)
        );
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            jim.variable_destruction_protocol(true),
            Some(VariableDestructionProtocol::CellContentsThenCallbacks)
        );
        let unknown_version = crate::InvocationDialect {
            tcl_version: None,
            ..c
        };
        assert_eq!(unknown_version.variable_destruction_protocol(true), None);
        let mixed_storage = crate::InvocationDialect {
            variable_container_model: Some(tcl_dialect::VariableContainerModel::DictionaryValue),
            ..c
        };
        assert_eq!(mixed_storage.variable_destruction_protocol(true), None);
    }
}
