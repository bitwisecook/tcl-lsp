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

// Physical output labels, never Tcl command lookup or publication authority.
use super::WasmModule;
use crate::native_lowering::NativeFunctionId;
use std::collections::{HashMap, HashSet};

pub(super) fn finish_owned_function_names(mut module: WasmModule) -> WasmModule {
    let mut collisions = {
        let mut counts = HashMap::new();
        for function in &module.functions {
            *counts.entry(function.name.as_str()).or_insert(0usize) += 1;
        }
        counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name.to_owned())
            .collect::<HashSet<_>>()
    };
    if !module.import_memory {
        collisions.insert("memory".to_owned());
    }
    // Reserve every original label before naming any body. A user label that
    // resembles a generated label must never capture a different function.
    let mut used = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();
    if !module.import_memory {
        used.insert("memory".to_owned());
    }
    for (index, owner) in &module.source_function_ids {
        let NativeFunctionId::Procedure(id) = owner else {
            continue;
        };
        let Some(position) = usize::try_from(*index)
            .ok()
            .and_then(|index| index.checked_sub(module.imports.len()))
        else {
            continue;
        };
        let Some(function) = module.functions.get_mut(position) else {
            continue;
        };
        if !collisions.contains(&function.name) {
            continue;
        }
        // The module-local ordinal is already an output-only typed body key.
        // Every colliding version gets its own symbol; no canonical Tcl name
        // is left pointing at an arbitrarily selected old or new definition.
        let mut name = format!("__tcl_wasm_body_{}", id.ordinal());
        while used.contains(&name) {
            name.push('_');
        }
        used.insert(name.clone());
        function.name = name;
    }
    module
}

#[cfg(test)]
mod tests {
    use super::{NativeFunctionId, WasmModule, finish_owned_function_names};

    #[test]
    fn colliding_versions_preserve_indices_and_avoid_reserved_user_labels() {
        let issuer = tcl_core_types::ProcedureIdIssuer::default();
        let first = issuer.allocate();
        let second = issuer.allocate();
        let mut module = WasmModule::new();
        module.add_import("host", "call", &[], &[]);
        for name in ["::top", "::top", "::top", "__tcl_wasm_body_0"] {
            let mut function = super::super::start_function(0, 0, None, 0);
            function.name = name.to_owned();
            module.functions.push(function);
        }
        module
            .source_function_ids
            .insert(1, NativeFunctionId::TopLevel);
        module
            .source_function_ids
            .insert(2, NativeFunctionId::Procedure(first));
        module
            .source_function_ids
            .insert(3, NativeFunctionId::Procedure(second));
        let original = module.clone();
        let mut renamed = finish_owned_function_names(module);
        assert_eq!(renamed.functions[0].name, "::top");
        assert_eq!(renamed.functions[1].name, "__tcl_wasm_body_0_");
        assert_eq!(renamed.functions[2].name, "__tcl_wasm_body_1");
        assert_eq!(renamed.functions[3].name, "__tcl_wasm_body_0");
        let mut restored = renamed.clone();
        for (function, before) in restored.functions.iter_mut().zip(&original.functions) {
            function.name.clone_from(&before.name);
        }
        assert_eq!(
            format!("{restored:?}"),
            format!("{original:?}"),
            "all non-name fields, indices and instructions remain exact"
        );
        assert_eq!(
            finish_owned_function_names(renamed.clone()).to_bytes(),
            renamed.to_bytes()
        );
    }
}
