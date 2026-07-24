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

//! `sdc_base` command specifications.

mod all_clocks;
mod all_fanin;
mod all_fanout;
mod all_inputs;
mod all_outputs;
mod all_registers;
mod append_to_collection;
mod check_timing;
mod create_clock;
mod create_generated_clock;
mod current_design;
mod define_proc_attributes;
mod filter_collection;
mod foreach_in_collection;
mod get_cells;
mod get_clocks;
mod get_lib_cells;
mod get_lib_pins;
mod get_libs;
mod get_nets;
mod get_object_name;
mod get_pins;
mod get_ports;
mod group_path;
mod link_design;
mod remove_from_collection;
mod report_area;
mod report_clock;
mod report_clock_timing;
mod report_constraint;
mod report_power;
mod report_timing;
mod set_case_analysis;
mod set_clock_groups;
mod set_clock_latency;
mod set_clock_transition;
mod set_clock_uncertainty;
mod set_disable_timing;
mod set_dont_touch;
mod set_dont_use;
mod set_driving_cell;
mod set_false_path;
mod set_ideal_latency;
mod set_ideal_network;
mod set_input_delay;
mod set_input_transition;
mod set_load;
mod set_max_area;
mod set_max_capacitance;
mod set_max_delay;
mod set_max_fanout;
mod set_max_transition;
mod set_min_delay;
mod set_multicycle_path;
mod set_output_delay;
mod set_propagated_clock;
mod set_size_only;
mod set_units;
mod set_wire_load_mode;
mod set_wire_load_model;
mod sizeof_collection;

use crate::spec::CommandSpec;

/// Return all `sdc_base` command specifications.
///
/// Every spec is stamped with the shared `sdc` [`required_package`]: the
/// industry-standard SDC constraint + collection library is a single package
/// ambient in every EDA profile (design doc `eda-library-packages.md`), rather
/// than being gated by a per-vendor dialect bit.
///
/// [`required_package`]: crate::spec::CommandSpec::required_package
#[must_use]
pub fn sdc_base_command_specs() -> Vec<CommandSpec> {
    let mut specs = vec![
        all_clocks::spec(),
        all_fanin::spec(),
        all_fanout::spec(),
        all_inputs::spec(),
        all_outputs::spec(),
        all_registers::spec(),
        append_to_collection::spec(),
        check_timing::spec(),
        create_clock::spec(),
        create_generated_clock::spec(),
        current_design::spec(),
        define_proc_attributes::spec(),
        filter_collection::spec(),
        foreach_in_collection::spec(),
        get_cells::spec(),
        get_clocks::spec(),
        get_lib_cells::spec(),
        get_lib_pins::spec(),
        get_libs::spec(),
        get_nets::spec(),
        get_object_name::spec(),
        get_pins::spec(),
        get_ports::spec(),
        group_path::spec(),
        link_design::spec(),
        remove_from_collection::spec(),
        report_area::spec(),
        report_clock::spec(),
        report_clock_timing::spec(),
        report_constraint::spec(),
        report_power::spec(),
        report_timing::spec(),
        set_case_analysis::spec(),
        set_clock_groups::spec(),
        set_clock_latency::spec(),
        set_clock_transition::spec(),
        set_clock_uncertainty::spec(),
        set_disable_timing::spec(),
        set_dont_touch::spec(),
        set_dont_use::spec(),
        set_driving_cell::spec(),
        set_false_path::spec(),
        set_ideal_latency::spec(),
        set_ideal_network::spec(),
        set_input_delay::spec(),
        set_input_transition::spec(),
        set_load::spec(),
        set_max_area::spec(),
        set_max_capacitance::spec(),
        set_max_delay::spec(),
        set_max_fanout::spec(),
        set_max_transition::spec(),
        set_min_delay::spec(),
        set_multicycle_path::spec(),
        set_output_delay::spec(),
        set_propagated_clock::spec(),
        set_size_only::spec(),
        set_units::spec(),
        set_wire_load_mode::spec(),
        set_wire_load_model::spec(),
        sizeof_collection::spec(),
    ];
    for spec in &mut specs {
        spec.required_package = Some("sdc");
    }
    specs
}
