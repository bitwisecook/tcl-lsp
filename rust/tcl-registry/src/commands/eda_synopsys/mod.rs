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

//! `eda_synopsys` command specifications.

mod analyze;
mod characterize;
mod check_design;
mod check_library;
mod clock_opt;
mod compile;
mod compile_ultra;
mod connect_net;
mod create_cell;
mod create_floorplan;
mod create_net;
mod create_port;
mod current_instance;
mod disconnect_net;
mod elaborate;
mod get_timing_paths;
mod group;
mod initialize_floorplan;
mod insert_clock_gating;
mod insert_dft;
mod link;
mod match_;
mod optimize_netlist;
mod place_opt;
mod printvar;
mod read_db;
mod read_ddc;
mod read_def;
mod read_file;
mod read_lef;
mod read_sdc;
mod read_verilog;
mod read_vhdl;
mod remove_cell;
mod remove_design;
mod report_analysis_coverage;
mod report_bottleneck;
mod report_cell;
mod report_clock_gating;
mod report_congestion;
mod report_delay_calculation;
mod report_design;
mod report_hierarchy;
mod report_net;
mod report_qor;
mod report_reference;
mod report_status;
mod route_auto;
mod route_opt;
mod set_app_var;
mod set_clock_gating_style;
mod set_host_options;
mod set_implementation_design;
mod set_operating_conditions;
mod set_reference_design;
mod set_scan_configuration;
mod set_technology;
mod size_cell;
mod swap_cell;
mod ungroup;
mod uniquify;
mod update_timing;
mod verify;
mod write;
mod write_def;
mod write_file;
mod write_gds;
mod write_sdc;

use crate::spec::CommandSpec;

/// The per-tool Synopsys package a command belongs to (design doc
/// `eda-library-packages.md`): Design Compiler is the bulk; `PrimeTime`, IC
/// Compiler II, and Formality are their own tool packages; the shell infra +
/// SDC I/O sit in the shared `synopsys` common package.
fn synopsys_package_for(name: &str) -> &'static str {
    match name {
        "get_timing_paths"
        | "update_timing"
        | "report_analysis_coverage"
        | "report_delay_calculation" => "synopsys-pt",
        "place_opt"
        | "clock_opt"
        | "route_opt"
        | "route_auto"
        | "create_floorplan"
        | "initialize_floorplan"
        | "read_def"
        | "read_lef"
        | "write_def"
        | "write_gds" => "synopsys-icc2",
        "set_reference_design" | "set_implementation_design" | "match" | "verify" => "synopsys-fm",
        "set_app_var" | "set_host_options" | "printvar" | "read_sdc" | "write_sdc" => "synopsys",
        _ => "synopsys-dc",
    }
}

/// Return all `eda_synopsys` command specifications.
#[must_use]
pub fn eda_synopsys_command_specs() -> Vec<CommandSpec> {
    let mut specs = vec![
        analyze::spec(),
        characterize::spec(),
        check_design::spec(),
        check_library::spec(),
        clock_opt::spec(),
        compile::spec(),
        compile_ultra::spec(),
        connect_net::spec(),
        create_cell::spec(),
        create_floorplan::spec(),
        create_net::spec(),
        create_port::spec(),
        current_instance::spec(),
        disconnect_net::spec(),
        elaborate::spec(),
        get_timing_paths::spec(),
        group::spec(),
        initialize_floorplan::spec(),
        insert_clock_gating::spec(),
        insert_dft::spec(),
        link::spec(),
        match_::spec(),
        optimize_netlist::spec(),
        place_opt::spec(),
        printvar::spec(),
        read_db::spec(),
        read_ddc::spec(),
        read_def::spec(),
        read_file::spec(),
        read_lef::spec(),
        read_sdc::spec(),
        read_verilog::spec(),
        read_vhdl::spec(),
        remove_cell::spec(),
        remove_design::spec(),
        report_analysis_coverage::spec(),
        report_bottleneck::spec(),
        report_cell::spec(),
        report_clock_gating::spec(),
        report_congestion::spec(),
        report_delay_calculation::spec(),
        report_design::spec(),
        report_hierarchy::spec(),
        report_net::spec(),
        report_qor::spec(),
        report_reference::spec(),
        report_status::spec(),
        route_auto::spec(),
        route_opt::spec(),
        set_app_var::spec(),
        set_clock_gating_style::spec(),
        set_host_options::spec(),
        set_implementation_design::spec(),
        set_operating_conditions::spec(),
        set_reference_design::spec(),
        set_scan_configuration::spec(),
        set_technology::spec(),
        size_cell::spec(),
        swap_cell::spec(),
        ungroup::spec(),
        uniquify::spec(),
        update_timing::spec(),
        verify::spec(),
        write::spec(),
        write_def::spec(),
        write_file::spec(),
        write_gds::spec(),
        write_sdc::spec(),
    ];
    for spec in &mut specs {
        spec.required_package = Some(synopsys_package_for(spec.name));
    }
    specs
}
