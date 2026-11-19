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

//! Source variable spellings, including scalar and array element boundaries.

mod array_elements;
mod variable_references;
pub use array_elements::{
    array_variable_completion_prefix, element_var_name, element_var_name_braced,
    element_var_name_braced_for_style, element_var_name_for_style, normalise_var_name_braced,
    normalise_var_name_braced_for_style, split_array_name, split_array_name_braced,
    split_array_name_braced_for_style, split_array_name_for_style, split_element_ref,
    split_element_ref_bytes,
};
pub use variable_references::{
    array_key_is_literal, is_static_variable_source_word, is_whole_variable_reference,
    normalise_var_name, normalise_var_name_for_style, split_braced_var_ref, var_reference,
    var_reference_for_style, variable_reference_root_bytes, whole_scalar_variable_reference_name,
    whole_variable_reference_literal_name,
};
