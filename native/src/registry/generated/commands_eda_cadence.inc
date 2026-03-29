// ── eda_cadence commands ──

// add_endcap
inline constexpr FormDesc eda_cadence_add_endcap_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_add_endcap_cmd{
    .name = "add_endcap",
    .forms = eda_cadence_add_endcap_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 1993,
};

// add_filler
inline constexpr FormDesc eda_cadence_add_filler_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_add_filler_cmd{
    .name = "add_filler",
    .forms = eda_cadence_add_filler_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 1994,
};

// add_well_tap
inline constexpr FormDesc eda_cadence_add_well_tap_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_add_well_tap_cmd{
    .name = "add_well_tap",
    .forms = eda_cadence_add_well_tap_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 1995,
};

// ccopt_design
inline constexpr FormDesc eda_cadence_ccopt_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_ccopt_design_cmd{
    .name = "ccopt_design",
    .forms = eda_cadence_ccopt_design_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 1996,
};

// check_design
inline constexpr FormDesc eda_cadence_check_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_check_design_cmd{
    .name = "check_design",
    .forms = eda_cadence_check_design_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 1997,
};

// check_timing_intent
inline constexpr FormDesc eda_cadence_check_timing_intent_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_check_timing_intent_cmd{
    .name = "check_timing_intent",
    .forms = eda_cadence_check_timing_intent_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 1998,
};

// create_analysis_view
inline constexpr FormDesc eda_cadence_create_analysis_view_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_create_analysis_view_cmd{
    .name = "create_analysis_view",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_cadence_create_analysis_view_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 1999,
};

// create_constraint_mode
inline constexpr FormDesc eda_cadence_create_constraint_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_create_constraint_mode_cmd{
    .name = "create_constraint_mode",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_cadence_create_constraint_mode_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2000,
};

// create_delay_corner
inline constexpr FormDesc eda_cadence_create_delay_corner_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_create_delay_corner_cmd{
    .name = "create_delay_corner",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_cadence_create_delay_corner_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2001,
};

// create_floorplan
inline constexpr FormDesc eda_cadence_create_floorplan_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_create_floorplan_cmd{
    .name = "create_floorplan",
    .forms = eda_cadence_create_floorplan_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2002,
};

// create_route_rule
inline constexpr FormDesc eda_cadence_create_route_rule_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_create_route_rule_cmd{
    .name = "create_route_rule",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_cadence_create_route_rule_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2003,
};

// dbGet
inline constexpr FormDesc eda_cadence_dbGet_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_dbGet_cmd{
    .name = "dbGet",
    .forms = eda_cadence_dbGet_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2004,
};

// dbQuery
inline constexpr FormDesc eda_cadence_dbQuery_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_dbQuery_cmd{
    .name = "dbQuery",
    .forms = eda_cadence_dbQuery_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2005,
};

// dbSet
inline constexpr FormDesc eda_cadence_dbSet_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_dbSet_cmd{
    .name = "dbSet",
    .forms = eda_cadence_dbSet_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2006,
};

// dbShape
inline constexpr FormDesc eda_cadence_dbShape_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_dbShape_cmd{
    .name = "dbShape",
    .forms = eda_cadence_dbShape_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2007,
};

// edit_pin
inline constexpr FormDesc eda_cadence_edit_pin_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_edit_pin_cmd{
    .name = "edit_pin",
    .forms = eda_cadence_edit_pin_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2008,
};

// elaborate
inline constexpr FormDesc eda_cadence_elaborate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_elaborate_cmd{
    .name = "elaborate",
    .forms = eda_cadence_elaborate_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2009,
};

// get_db
inline constexpr FormDesc eda_cadence_get_db_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_get_db_cmd{
    .name = "get_db",
    .forms = eda_cadence_get_db_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2010,
};

// init_design
inline constexpr FormDesc eda_cadence_init_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_init_design_cmd{
    .name = "init_design",
    .forms = eda_cadence_init_design_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2011,
};

// opt_design
inline constexpr FormDesc eda_cadence_opt_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_opt_design_cmd{
    .name = "opt_design",
    .forms = eda_cadence_opt_design_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2012,
};

// place_opt_design
inline constexpr FormDesc eda_cadence_place_opt_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_place_opt_design_cmd{
    .name = "place_opt_design",
    .forms = eda_cadence_place_opt_design_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2013,
};

// read_hdl
inline constexpr FormDesc eda_cadence_read_hdl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_read_hdl_cmd{
    .name = "read_hdl",
    .forms = eda_cadence_read_hdl_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2014,
};

// read_library
inline constexpr FormDesc eda_cadence_read_library_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_read_library_cmd{
    .name = "read_library",
    .forms = eda_cadence_read_library_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2015,
};

// read_mmmc
inline constexpr FormDesc eda_cadence_read_mmmc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_read_mmmc_cmd{
    .name = "read_mmmc",
    .arity = {1, 1},
    .forms = eda_cadence_read_mmmc_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2016,
};

// read_netlist
inline constexpr FormDesc eda_cadence_read_netlist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_read_netlist_cmd{
    .name = "read_netlist",
    .arity = {1, 1},
    .forms = eda_cadence_read_netlist_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2017,
};

// read_physical
inline constexpr FormDesc eda_cadence_read_physical_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_read_physical_cmd{
    .name = "read_physical",
    .forms = eda_cadence_read_physical_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2018,
};

// report_analysis_coverage
inline constexpr FormDesc eda_cadence_report_analysis_coverage_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_report_analysis_coverage_cmd{
    .name = "report_analysis_coverage",
    .forms = eda_cadence_report_analysis_coverage_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2019,
};

// report_area
inline constexpr FormDesc eda_cadence_report_area_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_report_area_cmd{
    .name = "report_area",
    .forms = eda_cadence_report_area_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2020,
};

// report_constraint
inline constexpr FormDesc eda_cadence_report_constraint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_report_constraint_cmd{
    .name = "report_constraint",
    .forms = eda_cadence_report_constraint_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2021,
};

// report_dp
inline constexpr FormDesc eda_cadence_report_dp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_report_dp_cmd{
    .name = "report_dp",
    .forms = eda_cadence_report_dp_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2022,
};

// report_gates
inline constexpr FormDesc eda_cadence_report_gates_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_report_gates_cmd{
    .name = "report_gates",
    .forms = eda_cadence_report_gates_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2023,
};

// report_power
inline constexpr FormDesc eda_cadence_report_power_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_report_power_cmd{
    .name = "report_power",
    .forms = eda_cadence_report_power_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2024,
};

// report_qor
inline constexpr FormDesc eda_cadence_report_qor_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_report_qor_cmd{
    .name = "report_qor",
    .forms = eda_cadence_report_qor_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2025,
};

// report_timing
inline constexpr FormDesc eda_cadence_report_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_report_timing_cmd{
    .name = "report_timing",
    .forms = eda_cadence_report_timing_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2026,
};

// route_design
inline constexpr FormDesc eda_cadence_route_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_route_design_cmd{
    .name = "route_design",
    .forms = eda_cadence_route_design_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2027,
};

// set_analysis_view
inline constexpr FormDesc eda_cadence_set_analysis_view_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_set_analysis_view_cmd{
    .name = "set_analysis_view",
    .forms = eda_cadence_set_analysis_view_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2028,
};

// set_db
inline constexpr FormDesc eda_cadence_set_db_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_set_db_cmd{
    .name = "set_db",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_cadence_set_db_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2029,
};

// stream_out
inline constexpr FormDesc eda_cadence_stream_out_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_stream_out_cmd{
    .name = "stream_out",
    .forms = eda_cadence_stream_out_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2030,
};

// syn_generic
inline constexpr FormDesc eda_cadence_syn_generic_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_syn_generic_cmd{
    .name = "syn_generic",
    .forms = eda_cadence_syn_generic_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2031,
};

// syn_map
inline constexpr FormDesc eda_cadence_syn_map_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_syn_map_cmd{
    .name = "syn_map",
    .forms = eda_cadence_syn_map_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2032,
};

// syn_opt
inline constexpr FormDesc eda_cadence_syn_opt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_syn_opt_cmd{
    .name = "syn_opt",
    .forms = eda_cadence_syn_opt_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2033,
};

// time_design
inline constexpr FormDesc eda_cadence_time_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_time_design_cmd{
    .name = "time_design",
    .forms = eda_cadence_time_design_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2034,
};

// update_timing
inline constexpr FormDesc eda_cadence_update_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_update_timing_cmd{
    .name = "update_timing",
    .forms = eda_cadence_update_timing_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2035,
};

// verify_connectivity
inline constexpr FormDesc eda_cadence_verify_connectivity_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_verify_connectivity_cmd{
    .name = "verify_connectivity",
    .forms = eda_cadence_verify_connectivity_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2036,
};

// verify_drc
inline constexpr FormDesc eda_cadence_verify_drc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_verify_drc_cmd{
    .name = "verify_drc",
    .forms = eda_cadence_verify_drc_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2037,
};

// verify_geometry
inline constexpr FormDesc eda_cadence_verify_geometry_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_verify_geometry_cmd{
    .name = "verify_geometry",
    .forms = eda_cadence_verify_geometry_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2038,
};

// write_def
inline constexpr FormDesc eda_cadence_write_def_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_write_def_cmd{
    .name = "write_def",
    .forms = eda_cadence_write_def_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2039,
};

// write_design
inline constexpr FormDesc eda_cadence_write_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_write_design_cmd{
    .name = "write_design",
    .forms = eda_cadence_write_design_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2040,
};

// write_do_lec
inline constexpr FormDesc eda_cadence_write_do_lec_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_write_do_lec_cmd{
    .name = "write_do_lec",
    .forms = eda_cadence_write_do_lec_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2041,
};

// write_gds
inline constexpr FormDesc eda_cadence_write_gds_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_write_gds_cmd{
    .name = "write_gds",
    .forms = eda_cadence_write_gds_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2042,
};

// write_hdl
inline constexpr FormDesc eda_cadence_write_hdl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_write_hdl_cmd{
    .name = "write_hdl",
    .forms = eda_cadence_write_hdl_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2043,
};

// write_netlist
inline constexpr FormDesc eda_cadence_write_netlist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_write_netlist_cmd{
    .name = "write_netlist",
    .forms = eda_cadence_write_netlist_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2044,
};

// write_sdc
inline constexpr FormDesc eda_cadence_write_sdc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_write_sdc_cmd{
    .name = "write_sdc",
    .forms = eda_cadence_write_sdc_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2045,
};

// xelab
inline constexpr FormDesc eda_cadence_xelab_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_xelab_cmd{
    .name = "xelab",
    .forms = eda_cadence_xelab_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2046,
};

// xrun
inline constexpr FormDesc eda_cadence_xrun_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_xrun_cmd{
    .name = "xrun",
    .forms = eda_cadence_xrun_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2047,
};

// xsim
inline constexpr FormDesc eda_cadence_xsim_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_cadence_xsim_cmd{
    .name = "xsim",
    .forms = eda_cadence_xsim_forms,
    .dialects = DialectFlags::EDA_CADENCE,
    .hover_index = 2048,
};
