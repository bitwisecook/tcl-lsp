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

//! `eda_quartus` command specifications.

mod check_timing;
mod close_device;
mod create_timing_netlist;
mod delete_timing_netlist;
mod derive_clocks;
mod derive_pll_clocks;
mod device_lock;
mod device_unlock;
mod execute_flow;
mod execute_module;
mod export_assignments;
mod get_all_assignments;
mod get_global_assignment;
mod get_instance_assignment;
mod get_io_assignment;
mod get_name_info;
mod get_names;
mod get_number_of_columns;
mod get_number_of_rows;
mod get_part_info;
mod get_part_list;
mod get_report_panel_data;
mod get_report_panel_id;
mod get_report_panel_row_index;
mod load_package;
mod load_report;
mod make_connection;
mod open_device;
mod project_close;
mod project_exists;
mod project_new;
mod project_open;
mod read_sdc;
mod remove_all_assignments;
mod remove_connection;
mod rename_node;
mod report_clock_fmax_summary;
mod report_datasheet;
mod report_min_pulse_width;
mod report_timing;
mod report_ucp;
mod save_report;
mod set_global_assignment;
mod set_instance_assignment;
mod set_io_assignment;
mod set_location_assignment;
mod set_parameter;
mod update_timing_netlist;

use crate::spec::CommandSpec;

/// The per-tool Quartus package a command belongs to, mirroring Intel's real
/// `::quartus::*` packages (design doc `eda-library-packages.md`): project +
/// assignments is the bulk default.
fn quartus_package_for(name: &str) -> &'static str {
    match name {
        "execute_flow" | "execute_module" => "quartus-flow",
        "create_timing_netlist"
        | "update_timing_netlist"
        | "delete_timing_netlist"
        | "check_timing"
        | "report_timing"
        | "report_clock_fmax_summary"
        | "report_datasheet"
        | "report_min_pulse_width"
        | "report_ucp"
        | "read_sdc" => "quartus-sta",
        "derive_clocks" | "derive_pll_clocks" => "quartus-sdc-ext",
        "load_report"
        | "save_report"
        | "get_report_panel_data"
        | "get_report_panel_id"
        | "get_report_panel_row_index"
        | "get_number_of_columns"
        | "get_number_of_rows" => "quartus-report",
        "device_lock" | "device_unlock" | "open_device" | "close_device" | "get_part_info"
        | "get_part_list" => "quartus-device",
        "load_package" => "quartus-misc",
        _ => "quartus-project",
    }
}

/// Return all `eda_quartus` command specifications.
#[must_use]
pub fn eda_quartus_command_specs() -> Vec<CommandSpec> {
    let mut specs = vec![
        check_timing::spec(),
        close_device::spec(),
        create_timing_netlist::spec(),
        delete_timing_netlist::spec(),
        derive_clocks::spec(),
        derive_pll_clocks::spec(),
        device_lock::spec(),
        device_unlock::spec(),
        execute_flow::spec(),
        execute_module::spec(),
        export_assignments::spec(),
        get_all_assignments::spec(),
        get_global_assignment::spec(),
        get_instance_assignment::spec(),
        get_io_assignment::spec(),
        get_name_info::spec(),
        get_names::spec(),
        get_number_of_columns::spec(),
        get_number_of_rows::spec(),
        get_part_info::spec(),
        get_part_list::spec(),
        get_report_panel_data::spec(),
        get_report_panel_id::spec(),
        get_report_panel_row_index::spec(),
        load_package::spec(),
        load_report::spec(),
        make_connection::spec(),
        open_device::spec(),
        project_close::spec(),
        project_exists::spec(),
        project_new::spec(),
        project_open::spec(),
        read_sdc::spec(),
        remove_all_assignments::spec(),
        remove_connection::spec(),
        rename_node::spec(),
        report_clock_fmax_summary::spec(),
        report_datasheet::spec(),
        report_min_pulse_width::spec(),
        report_timing::spec(),
        report_ucp::spec(),
        save_report::spec(),
        set_global_assignment::spec(),
        set_instance_assignment::spec(),
        set_io_assignment::spec(),
        set_location_assignment::spec(),
        set_parameter::spec(),
        update_timing_netlist::spec(),
    ];
    for spec in &mut specs {
        spec.required_package = Some(quartus_package_for(spec.name));
    }
    specs
}
