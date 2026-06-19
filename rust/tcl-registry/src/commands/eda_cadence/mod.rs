//! `eda_cadence` command specifications.

mod add_endcap;
mod add_filler;
mod add_well_tap;
mod ccopt_design;
mod check_design;
mod check_timing_intent;
mod create_analysis_view;
mod create_constraint_mode;
mod create_delay_corner;
mod create_floorplan;
mod create_route_rule;
mod dbget;
mod dbquery;
mod dbset;
mod dbshape;
mod edit_pin;
mod elaborate;
mod get_db;
mod init_design;
mod opt_design;
mod place_opt_design;
mod read_hdl;
mod read_library;
mod read_mmmc;
mod read_netlist;
mod read_physical;
mod report_analysis_coverage;
mod report_area;
mod report_constraint;
mod report_dp;
mod report_gates;
mod report_power;
mod report_qor;
mod report_timing;
mod route_design;
mod set_analysis_view;
mod set_db;
mod stream_out;
mod syn_generic;
mod syn_map;
mod syn_opt;
mod time_design;
mod update_timing;
mod verify_connectivity;
mod verify_drc;
mod verify_geometry;
mod write_def;
mod write_design;
mod write_do_lec;
mod write_gds;
mod write_hdl;
mod write_netlist;
mod write_sdc;
mod xelab;
mod xrun;
mod xsim;

use crate::spec::CommandSpec;

/// Return all `eda_cadence` command specifications.
#[must_use]
pub fn eda_cadence_command_specs() -> Vec<CommandSpec> {
    vec![
        add_endcap::spec(),
        add_filler::spec(),
        add_well_tap::spec(),
        ccopt_design::spec(),
        check_design::spec(),
        check_timing_intent::spec(),
        create_analysis_view::spec(),
        create_constraint_mode::spec(),
        create_delay_corner::spec(),
        create_floorplan::spec(),
        create_route_rule::spec(),
        dbget::spec(),
        dbquery::spec(),
        dbset::spec(),
        dbshape::spec(),
        edit_pin::spec(),
        elaborate::spec(),
        get_db::spec(),
        init_design::spec(),
        opt_design::spec(),
        place_opt_design::spec(),
        read_hdl::spec(),
        read_library::spec(),
        read_mmmc::spec(),
        read_netlist::spec(),
        read_physical::spec(),
        report_analysis_coverage::spec(),
        report_area::spec(),
        report_constraint::spec(),
        report_dp::spec(),
        report_gates::spec(),
        report_power::spec(),
        report_qor::spec(),
        report_timing::spec(),
        route_design::spec(),
        set_analysis_view::spec(),
        set_db::spec(),
        stream_out::spec(),
        syn_generic::spec(),
        syn_map::spec(),
        syn_opt::spec(),
        time_design::spec(),
        update_timing::spec(),
        verify_connectivity::spec(),
        verify_drc::spec(),
        verify_geometry::spec(),
        write_def::spec(),
        write_design::spec(),
        write_do_lec::spec(),
        write_gds::spec(),
        write_hdl::spec(),
        write_netlist::spec(),
        write_sdc::spec(),
        xelab::spec(),
        xrun::spec(),
        xsim::spec(),
    ]
}
