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

//! `eda_xilinx` command specifications.

#![allow(non_snake_case)]

mod apply_bd_automation;
mod close_hw_manager;
mod close_project;
mod close_sim;
mod config_ip_cache;
mod connect_bd_intf_net;
mod connect_bd_net;
mod connect_hw_server;
mod create_bd_cell;
mod create_bd_design;
mod create_bd_intf_port;
mod create_bd_port;
mod create_ip;
mod create_project;
mod create_run;
mod current_project;
mod generate_target;
mod get_ips;
mod get_projects;
mod get_property;
mod get_runs;
mod import_ip;
mod ipx__add_bus_interface;
mod ipx__package_project;
mod launch_runs;
mod launch_simulation;
mod open_bd_design;
mod open_checkpoint;
mod open_hw_manager;
mod open_hw_target;
mod open_project;
mod open_run;
mod opt_design;
mod phys_opt_design;
mod place_design;
mod program_hw_devices;
mod read_checkpoint;
mod read_edif;
mod read_ip;
mod read_verilog;
mod read_vhdl;
mod read_xdc;
mod report_clock_networks;
mod report_clock_utilization;
mod report_design_analysis;
mod report_drc;
mod report_io;
mod report_methodology;
mod report_power;
mod report_route_status;
mod report_timing;
mod report_timing_summary;
mod report_utilization;
mod reset_run;
mod route_design;
mod save_bd_design;
mod save_project_as;
mod set_property;
mod synth_design;
mod upgrade_ip;
mod validate_bd_design;
mod wait_on_run;
mod write_bitstream;
mod write_checkpoint;

use crate::spec::CommandSpec;

/// Return all `eda_xilinx` command specifications.
#[must_use]
pub fn eda_xilinx_command_specs() -> Vec<CommandSpec> {
    let mut specs = vec![
        apply_bd_automation::spec(),
        close_hw_manager::spec(),
        close_project::spec(),
        close_sim::spec(),
        config_ip_cache::spec(),
        connect_bd_intf_net::spec(),
        connect_bd_net::spec(),
        connect_hw_server::spec(),
        create_bd_cell::spec(),
        create_bd_design::spec(),
        create_bd_intf_port::spec(),
        create_bd_port::spec(),
        create_ip::spec(),
        create_project::spec(),
        create_run::spec(),
        current_project::spec(),
        generate_target::spec(),
        get_ips::spec(),
        get_projects::spec(),
        get_property::spec(),
        get_runs::spec(),
        import_ip::spec(),
        ipx__add_bus_interface::spec(),
        ipx__package_project::spec(),
        launch_runs::spec(),
        launch_simulation::spec(),
        open_bd_design::spec(),
        open_checkpoint::spec(),
        open_hw_manager::spec(),
        open_hw_target::spec(),
        open_project::spec(),
        open_run::spec(),
        opt_design::spec(),
        phys_opt_design::spec(),
        place_design::spec(),
        program_hw_devices::spec(),
        read_checkpoint::spec(),
        read_edif::spec(),
        read_ip::spec(),
        read_verilog::spec(),
        read_vhdl::spec(),
        read_xdc::spec(),
        report_clock_networks::spec(),
        report_clock_utilization::spec(),
        report_design_analysis::spec(),
        report_drc::spec(),
        report_io::spec(),
        report_methodology::spec(),
        report_power::spec(),
        report_route_status::spec(),
        report_timing::spec(),
        report_timing_summary::spec(),
        report_utilization::spec(),
        reset_run::spec(),
        route_design::spec(),
        save_bd_design::spec(),
        save_project_as::spec(),
        set_property::spec(),
        synth_design::spec(),
        upgrade_ip::spec(),
        validate_bd_design::spec(),
        wait_on_run::spec(),
        write_bitstream::spec(),
        write_checkpoint::spec(),
    ];
    for spec in &mut specs {
        spec.required_package = Some("vivado");
    }
    specs
}
