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

//! Authoritative field inventory for durable compiler units.
//!
//! Each inventory and exhaustive field witness share a declaration in the owner
//! crate, so private retained inputs are covered without exposing their values.
//! Consumers must explicitly account for every published field name; these
//! names grant no source, configuration, or native execution authority.

use crate::compilation_unit::{CompilationUnit, FunctionUnit};

macro_rules! durable_inventory {
    ($witness:ident, $fields:ident, $ty:ident, $($field:ident),+ $(,)?) => {
        /// Every durable field of the type, as `Type::field`.
        pub const $fields: &[&str] =
            &[$(concat!(stringify!($ty), "::", stringify!($field))),+];

        fn $witness(value: &$ty) {
            let $ty { $($field: _),+ } = value;
        }
    };
}

durable_inventory!(
    witness_function_unit,
    FUNCTION_UNIT_FIELDS,
    FunctionUnit,
    source_metadata_input,
    source_config,
    name,
    cfg,
    ssa,
    def_use,
    sccp,
    semantic_value_projection,
    types,
    return_type,
    taints,
    rendered_props,
    memory_ssa,
    dynamic_names,
    complexity_guarded,
    base_offset,
    method_facts,
    irules_event_body,
    semantic_facts,
);

durable_inventory!(
    witness_compilation_unit,
    COMPILATION_UNIT_FIELDS,
    CompilationUnit,
    source,
    ir_module,
    cfg_module,
    command_mutations,
    top_level,
    procedures,
    methods,
    body_units,
    interproc,
    connection_scope,
    caller_scope,
    declared_commands,
);

/// Read-only exhaustive witness for both durable compiler types.
///
/// The field-name inventory and each no-`..` destructure share one declaration.
/// Private retained inputs remain private; consumers review their names through
/// the inventory without obtaining authority to replace their values.
pub fn assert_durable_field_inventory(unit: &CompilationUnit) {
    witness_function_unit(&unit.top_level);
    witness_compilation_unit(unit);
}
