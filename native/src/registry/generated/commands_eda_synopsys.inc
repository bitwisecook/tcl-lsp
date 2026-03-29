// ── eda_synopsys commands ──

// analyze
inline constexpr FormDesc eda_synopsys_analyze_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_analyze_cmd{
    .name = "analyze",
    .forms = eda_synopsys_analyze_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1925,
};

// characterize
inline constexpr FormDesc eda_synopsys_characterize_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_characterize_cmd{
    .name = "characterize",
    .forms = eda_synopsys_characterize_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1926,
};

// check_design
inline constexpr FormDesc eda_synopsys_check_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_check_design_cmd{
    .name = "check_design",
    .forms = eda_synopsys_check_design_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1927,
};

// check_library
inline constexpr FormDesc eda_synopsys_check_library_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_check_library_cmd{
    .name = "check_library",
    .forms = eda_synopsys_check_library_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1928,
};

// clock_opt
inline constexpr FormDesc eda_synopsys_clock_opt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_clock_opt_cmd{
    .name = "clock_opt",
    .forms = eda_synopsys_clock_opt_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1929,
};

// compile
inline constexpr FormDesc eda_synopsys_compile_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_compile_cmd{
    .name = "compile",
    .forms = eda_synopsys_compile_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1930,
};

// compile_ultra
inline constexpr FormDesc eda_synopsys_compile_ultra_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_compile_ultra_cmd{
    .name = "compile_ultra",
    .forms = eda_synopsys_compile_ultra_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1931,
};

// connect_net
inline constexpr FormDesc eda_synopsys_connect_net_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_connect_net_cmd{
    .name = "connect_net",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_synopsys_connect_net_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1932,
};

// create_cell
inline constexpr FormDesc eda_synopsys_create_cell_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_create_cell_cmd{
    .name = "create_cell",
    .arity = {2, 2},
    .forms = eda_synopsys_create_cell_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1933,
};

// create_floorplan
inline constexpr FormDesc eda_synopsys_create_floorplan_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_create_floorplan_cmd{
    .name = "create_floorplan",
    .forms = eda_synopsys_create_floorplan_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1934,
};

// create_net
inline constexpr FormDesc eda_synopsys_create_net_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_create_net_cmd{
    .name = "create_net",
    .arity = {1, 1},
    .forms = eda_synopsys_create_net_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1935,
};

// create_port
inline constexpr FormDesc eda_synopsys_create_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_create_port_cmd{
    .name = "create_port",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_synopsys_create_port_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1936,
};

// current_instance
inline constexpr FormDesc eda_synopsys_current_instance_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_current_instance_cmd{
    .name = "current_instance",
    .forms = eda_synopsys_current_instance_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1937,
};

// disconnect_net
inline constexpr FormDesc eda_synopsys_disconnect_net_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_disconnect_net_cmd{
    .name = "disconnect_net",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_synopsys_disconnect_net_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1938,
};

// elaborate
inline constexpr FormDesc eda_synopsys_elaborate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_elaborate_cmd{
    .name = "elaborate",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_synopsys_elaborate_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1939,
};

// get_timing_paths
inline constexpr FormDesc eda_synopsys_get_timing_paths_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_get_timing_paths_cmd{
    .name = "get_timing_paths",
    .forms = eda_synopsys_get_timing_paths_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1940,
};

// group
inline constexpr FormDesc eda_synopsys_group_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_group_cmd{
    .name = "group",
    .forms = eda_synopsys_group_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1941,
};

// initialize_floorplan
inline constexpr FormDesc eda_synopsys_initialize_floorplan_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_initialize_floorplan_cmd{
    .name = "initialize_floorplan",
    .forms = eda_synopsys_initialize_floorplan_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1942,
};

// insert_clock_gating
inline constexpr FormDesc eda_synopsys_insert_clock_gating_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_insert_clock_gating_cmd{
    .name = "insert_clock_gating",
    .forms = eda_synopsys_insert_clock_gating_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1943,
};

// insert_dft
inline constexpr FormDesc eda_synopsys_insert_dft_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_insert_dft_cmd{
    .name = "insert_dft",
    .forms = eda_synopsys_insert_dft_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1944,
};

// link
inline constexpr FormDesc eda_synopsys_link_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_link_cmd{
    .name = "link",
    .forms = eda_synopsys_link_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1945,
};

// match
inline constexpr FormDesc eda_synopsys_match_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_match_cmd{
    .name = "match",
    .forms = eda_synopsys_match_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1946,
};

// optimize_netlist
inline constexpr FormDesc eda_synopsys_optimize_netlist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_optimize_netlist_cmd{
    .name = "optimize_netlist",
    .forms = eda_synopsys_optimize_netlist_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1947,
};

// place_opt
inline constexpr FormDesc eda_synopsys_place_opt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_place_opt_cmd{
    .name = "place_opt",
    .forms = eda_synopsys_place_opt_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1948,
};

// printvar
inline constexpr FormDesc eda_synopsys_printvar_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_printvar_cmd{
    .name = "printvar",
    .forms = eda_synopsys_printvar_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1949,
};

// read_db
inline constexpr FormDesc eda_synopsys_read_db_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_read_db_cmd{
    .name = "read_db",
    .arity = {1, 1},
    .forms = eda_synopsys_read_db_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1950,
};

// read_ddc
inline constexpr FormDesc eda_synopsys_read_ddc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_read_ddc_cmd{
    .name = "read_ddc",
    .arity = {1, 1},
    .forms = eda_synopsys_read_ddc_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1951,
};

// read_def
inline constexpr FormDesc eda_synopsys_read_def_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_read_def_cmd{
    .name = "read_def",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_synopsys_read_def_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1952,
};

// read_file
inline constexpr FormDesc eda_synopsys_read_file_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_read_file_cmd{
    .name = "read_file",
    .forms = eda_synopsys_read_file_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1953,
};

// read_lef
inline constexpr FormDesc eda_synopsys_read_lef_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_read_lef_cmd{
    .name = "read_lef",
    .arity = {1, 1},
    .forms = eda_synopsys_read_lef_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1954,
};

// read_sdc
inline constexpr FormDesc eda_synopsys_read_sdc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_read_sdc_cmd{
    .name = "read_sdc",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_synopsys_read_sdc_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1955,
};

// read_verilog
inline constexpr FormDesc eda_synopsys_read_verilog_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_read_verilog_cmd{
    .name = "read_verilog",
    .forms = eda_synopsys_read_verilog_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1956,
};

// read_vhdl
inline constexpr FormDesc eda_synopsys_read_vhdl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_read_vhdl_cmd{
    .name = "read_vhdl",
    .forms = eda_synopsys_read_vhdl_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1957,
};

// remove_cell
inline constexpr FormDesc eda_synopsys_remove_cell_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_remove_cell_cmd{
    .name = "remove_cell",
    .forms = eda_synopsys_remove_cell_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1958,
};

// remove_design
inline constexpr FormDesc eda_synopsys_remove_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_remove_design_cmd{
    .name = "remove_design",
    .forms = eda_synopsys_remove_design_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1959,
};

// report_analysis_coverage
inline constexpr FormDesc eda_synopsys_report_analysis_coverage_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_analysis_coverage_cmd{
    .name = "report_analysis_coverage",
    .forms = eda_synopsys_report_analysis_coverage_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1960,
};

// report_bottleneck
inline constexpr FormDesc eda_synopsys_report_bottleneck_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_bottleneck_cmd{
    .name = "report_bottleneck",
    .forms = eda_synopsys_report_bottleneck_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1961,
};

// report_cell
inline constexpr FormDesc eda_synopsys_report_cell_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_cell_cmd{
    .name = "report_cell",
    .forms = eda_synopsys_report_cell_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1962,
};

// report_clock_gating
inline constexpr FormDesc eda_synopsys_report_clock_gating_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_clock_gating_cmd{
    .name = "report_clock_gating",
    .forms = eda_synopsys_report_clock_gating_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1963,
};

// report_congestion
inline constexpr FormDesc eda_synopsys_report_congestion_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_congestion_cmd{
    .name = "report_congestion",
    .forms = eda_synopsys_report_congestion_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1964,
};

// report_delay_calculation
inline constexpr FormDesc eda_synopsys_report_delay_calculation_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_delay_calculation_cmd{
    .name = "report_delay_calculation",
    .forms = eda_synopsys_report_delay_calculation_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1965,
};

// report_design
inline constexpr FormDesc eda_synopsys_report_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_design_cmd{
    .name = "report_design",
    .forms = eda_synopsys_report_design_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1966,
};

// report_hierarchy
inline constexpr FormDesc eda_synopsys_report_hierarchy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_hierarchy_cmd{
    .name = "report_hierarchy",
    .forms = eda_synopsys_report_hierarchy_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1967,
};

// report_net
inline constexpr FormDesc eda_synopsys_report_net_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_net_cmd{
    .name = "report_net",
    .forms = eda_synopsys_report_net_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1968,
};

// report_qor
inline constexpr FormDesc eda_synopsys_report_qor_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_qor_cmd{
    .name = "report_qor",
    .forms = eda_synopsys_report_qor_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1969,
};

// report_reference
inline constexpr FormDesc eda_synopsys_report_reference_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_reference_cmd{
    .name = "report_reference",
    .forms = eda_synopsys_report_reference_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1970,
};

// report_status
inline constexpr FormDesc eda_synopsys_report_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_report_status_cmd{
    .name = "report_status",
    .forms = eda_synopsys_report_status_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1971,
};

// route_auto
inline constexpr FormDesc eda_synopsys_route_auto_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_route_auto_cmd{
    .name = "route_auto",
    .forms = eda_synopsys_route_auto_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1972,
};

// route_opt
inline constexpr FormDesc eda_synopsys_route_opt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_route_opt_cmd{
    .name = "route_opt",
    .forms = eda_synopsys_route_opt_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1973,
};

// set_app_var
inline constexpr FormDesc eda_synopsys_set_app_var_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_set_app_var_cmd{
    .name = "set_app_var",
    .arity = {2, 2},
    .forms = eda_synopsys_set_app_var_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1974,
};

// set_clock_gating_style
inline constexpr FormDesc eda_synopsys_set_clock_gating_style_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_set_clock_gating_style_cmd{
    .name = "set_clock_gating_style",
    .forms = eda_synopsys_set_clock_gating_style_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1975,
};

// set_host_options
inline constexpr FormDesc eda_synopsys_set_host_options_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_set_host_options_cmd{
    .name = "set_host_options",
    .forms = eda_synopsys_set_host_options_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1976,
};

// set_implementation_design
inline constexpr FormDesc eda_synopsys_set_implementation_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_set_implementation_design_cmd{
    .name = "set_implementation_design",
    .arity = {1, 1},
    .forms = eda_synopsys_set_implementation_design_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1977,
};

// set_operating_conditions
inline constexpr FormDesc eda_synopsys_set_operating_conditions_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_set_operating_conditions_cmd{
    .name = "set_operating_conditions",
    .forms = eda_synopsys_set_operating_conditions_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1978,
};

// set_reference_design
inline constexpr FormDesc eda_synopsys_set_reference_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_set_reference_design_cmd{
    .name = "set_reference_design",
    .arity = {1, 1},
    .forms = eda_synopsys_set_reference_design_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1979,
};

// set_scan_configuration
inline constexpr FormDesc eda_synopsys_set_scan_configuration_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_set_scan_configuration_cmd{
    .name = "set_scan_configuration",
    .forms = eda_synopsys_set_scan_configuration_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1980,
};

// set_technology
inline constexpr FormDesc eda_synopsys_set_technology_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_set_technology_cmd{
    .name = "set_technology",
    .forms = eda_synopsys_set_technology_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1981,
};

// size_cell
inline constexpr FormDesc eda_synopsys_size_cell_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_size_cell_cmd{
    .name = "size_cell",
    .arity = {2, 2},
    .forms = eda_synopsys_size_cell_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1982,
};

// swap_cell
inline constexpr FormDesc eda_synopsys_swap_cell_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_swap_cell_cmd{
    .name = "swap_cell",
    .forms = eda_synopsys_swap_cell_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1983,
};

// ungroup
inline constexpr FormDesc eda_synopsys_ungroup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_ungroup_cmd{
    .name = "ungroup",
    .forms = eda_synopsys_ungroup_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1984,
};

// uniquify
inline constexpr FormDesc eda_synopsys_uniquify_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_uniquify_cmd{
    .name = "uniquify",
    .forms = eda_synopsys_uniquify_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1985,
};

// update_timing
inline constexpr FormDesc eda_synopsys_update_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_update_timing_cmd{
    .name = "update_timing",
    .forms = eda_synopsys_update_timing_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1986,
};

// verify
inline constexpr FormDesc eda_synopsys_verify_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_verify_cmd{
    .name = "verify",
    .forms = eda_synopsys_verify_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1987,
};

// write
inline constexpr FormDesc eda_synopsys_write_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_write_cmd{
    .name = "write",
    .forms = eda_synopsys_write_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1988,
};

// write_def
inline constexpr FormDesc eda_synopsys_write_def_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_write_def_cmd{
    .name = "write_def",
    .forms = eda_synopsys_write_def_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1989,
};

// write_file
inline constexpr FormDesc eda_synopsys_write_file_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_write_file_cmd{
    .name = "write_file",
    .forms = eda_synopsys_write_file_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1990,
};

// write_gds
inline constexpr FormDesc eda_synopsys_write_gds_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_write_gds_cmd{
    .name = "write_gds",
    .forms = eda_synopsys_write_gds_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1991,
};

// write_sdc
inline constexpr FormDesc eda_synopsys_write_sdc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_synopsys_write_sdc_cmd{
    .name = "write_sdc",
    .forms = eda_synopsys_write_sdc_forms,
    .dialects = DialectFlags::EDA_SYNOPSYS,
    .hover_index = 1992,
};
