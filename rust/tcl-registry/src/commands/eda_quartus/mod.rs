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

/// Return all `eda_quartus` command specifications.
#[must_use]
pub fn eda_quartus_command_specs() -> Vec<CommandSpec> {
    vec![
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
    ]
}
