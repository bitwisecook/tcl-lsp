// ── eda_quartus commands ──

// check_timing
inline constexpr FormDesc eda_quartus_check_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_check_timing_cmd{
    .name = "check_timing",
    .forms = eda_quartus_check_timing_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2113,
};

// close_device
inline constexpr FormDesc eda_quartus_close_device_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_close_device_cmd{
    .name = "close_device",
    .forms = eda_quartus_close_device_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2114,
};

// create_timing_netlist
inline constexpr FormDesc eda_quartus_create_timing_netlist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_create_timing_netlist_cmd{
    .name = "create_timing_netlist",
    .forms = eda_quartus_create_timing_netlist_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2115,
};

// delete_timing_netlist
inline constexpr FormDesc eda_quartus_delete_timing_netlist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_delete_timing_netlist_cmd{
    .name = "delete_timing_netlist",
    .forms = eda_quartus_delete_timing_netlist_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2116,
};

// derive_clocks
inline constexpr FormDesc eda_quartus_derive_clocks_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_derive_clocks_cmd{
    .name = "derive_clocks",
    .forms = eda_quartus_derive_clocks_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2117,
};

// derive_pll_clocks
inline constexpr FormDesc eda_quartus_derive_pll_clocks_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_derive_pll_clocks_cmd{
    .name = "derive_pll_clocks",
    .forms = eda_quartus_derive_pll_clocks_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2118,
};

// device_lock
inline constexpr FormDesc eda_quartus_device_lock_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_device_lock_cmd{
    .name = "device_lock",
    .forms = eda_quartus_device_lock_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2119,
};

// device_unlock
inline constexpr FormDesc eda_quartus_device_unlock_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_device_unlock_cmd{
    .name = "device_unlock",
    .forms = eda_quartus_device_unlock_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2120,
};

// execute_flow
inline constexpr FormDesc eda_quartus_execute_flow_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_execute_flow_cmd{
    .name = "execute_flow",
    .forms = eda_quartus_execute_flow_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2121,
};

// execute_module
inline constexpr FormDesc eda_quartus_execute_module_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_execute_module_cmd{
    .name = "execute_module",
    .forms = eda_quartus_execute_module_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2122,
};

// export_assignments
inline constexpr FormDesc eda_quartus_export_assignments_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_export_assignments_cmd{
    .name = "export_assignments",
    .forms = eda_quartus_export_assignments_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2123,
};

// get_all_assignments
inline constexpr FormDesc eda_quartus_get_all_assignments_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_all_assignments_cmd{
    .name = "get_all_assignments",
    .forms = eda_quartus_get_all_assignments_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2124,
};

// get_global_assignment
inline constexpr FormDesc eda_quartus_get_global_assignment_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_global_assignment_cmd{
    .name = "get_global_assignment",
    .forms = eda_quartus_get_global_assignment_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2125,
};

// get_instance_assignment
inline constexpr FormDesc eda_quartus_get_instance_assignment_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_instance_assignment_cmd{
    .name = "get_instance_assignment",
    .forms = eda_quartus_get_instance_assignment_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2126,
};

// get_io_assignment
inline constexpr FormDesc eda_quartus_get_io_assignment_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_io_assignment_cmd{
    .name = "get_io_assignment",
    .forms = eda_quartus_get_io_assignment_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2127,
};

// get_name_info
inline constexpr FormDesc eda_quartus_get_name_info_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_name_info_cmd{
    .name = "get_name_info",
    .forms = eda_quartus_get_name_info_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2128,
};

// get_names
inline constexpr FormDesc eda_quartus_get_names_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_names_cmd{
    .name = "get_names",
    .forms = eda_quartus_get_names_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2129,
};

// get_number_of_columns
inline constexpr FormDesc eda_quartus_get_number_of_columns_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_number_of_columns_cmd{
    .name = "get_number_of_columns",
    .forms = eda_quartus_get_number_of_columns_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2130,
};

// get_number_of_rows
inline constexpr FormDesc eda_quartus_get_number_of_rows_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_number_of_rows_cmd{
    .name = "get_number_of_rows",
    .forms = eda_quartus_get_number_of_rows_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2131,
};

// get_part_info
inline constexpr FormDesc eda_quartus_get_part_info_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_part_info_cmd{
    .name = "get_part_info",
    .forms = eda_quartus_get_part_info_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2132,
};

// get_part_list
inline constexpr FormDesc eda_quartus_get_part_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_part_list_cmd{
    .name = "get_part_list",
    .forms = eda_quartus_get_part_list_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2133,
};

// get_report_panel_data
inline constexpr FormDesc eda_quartus_get_report_panel_data_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_report_panel_data_cmd{
    .name = "get_report_panel_data",
    .forms = eda_quartus_get_report_panel_data_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2134,
};

// get_report_panel_id
inline constexpr FormDesc eda_quartus_get_report_panel_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_report_panel_id_cmd{
    .name = "get_report_panel_id",
    .arity = {1, 1},
    .forms = eda_quartus_get_report_panel_id_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2135,
};

// get_report_panel_row_index
inline constexpr FormDesc eda_quartus_get_report_panel_row_index_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_get_report_panel_row_index_cmd{
    .name = "get_report_panel_row_index",
    .forms = eda_quartus_get_report_panel_row_index_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2136,
};

// load_package
inline constexpr FormDesc eda_quartus_load_package_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_load_package_cmd{
    .name = "load_package",
    .arity = {1, 1},
    .forms = eda_quartus_load_package_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2137,
};

// load_report
inline constexpr FormDesc eda_quartus_load_report_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_load_report_cmd{
    .name = "load_report",
    .forms = eda_quartus_load_report_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2138,
};

// make_connection
inline constexpr FormDesc eda_quartus_make_connection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_make_connection_cmd{
    .name = "make_connection",
    .forms = eda_quartus_make_connection_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2139,
};

// open_device
inline constexpr FormDesc eda_quartus_open_device_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_open_device_cmd{
    .name = "open_device",
    .forms = eda_quartus_open_device_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2140,
};

// project_close
inline constexpr FormDesc eda_quartus_project_close_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_project_close_cmd{
    .name = "project_close",
    .forms = eda_quartus_project_close_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2141,
};

// project_exists
inline constexpr FormDesc eda_quartus_project_exists_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_project_exists_cmd{
    .name = "project_exists",
    .arity = {1, 1},
    .forms = eda_quartus_project_exists_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2142,
};

// project_new
inline constexpr FormDesc eda_quartus_project_new_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_project_new_cmd{
    .name = "project_new",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_quartus_project_new_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2143,
};

// project_open
inline constexpr FormDesc eda_quartus_project_open_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_project_open_cmd{
    .name = "project_open",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_quartus_project_open_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2144,
};

// read_sdc
inline constexpr FormDesc eda_quartus_read_sdc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_read_sdc_cmd{
    .name = "read_sdc",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_quartus_read_sdc_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2145,
};

// remove_all_assignments
inline constexpr FormDesc eda_quartus_remove_all_assignments_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_remove_all_assignments_cmd{
    .name = "remove_all_assignments",
    .forms = eda_quartus_remove_all_assignments_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2146,
};

// remove_connection
inline constexpr FormDesc eda_quartus_remove_connection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_remove_connection_cmd{
    .name = "remove_connection",
    .forms = eda_quartus_remove_connection_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2147,
};

// rename_node
inline constexpr FormDesc eda_quartus_rename_node_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_rename_node_cmd{
    .name = "rename_node",
    .forms = eda_quartus_rename_node_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2148,
};

// report_clock_fmax_summary
inline constexpr FormDesc eda_quartus_report_clock_fmax_summary_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_report_clock_fmax_summary_cmd{
    .name = "report_clock_fmax_summary",
    .forms = eda_quartus_report_clock_fmax_summary_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2149,
};

// report_datasheet
inline constexpr FormDesc eda_quartus_report_datasheet_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_report_datasheet_cmd{
    .name = "report_datasheet",
    .forms = eda_quartus_report_datasheet_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2150,
};

// report_min_pulse_width
inline constexpr FormDesc eda_quartus_report_min_pulse_width_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_report_min_pulse_width_cmd{
    .name = "report_min_pulse_width",
    .forms = eda_quartus_report_min_pulse_width_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2151,
};

// report_timing
inline constexpr FormDesc eda_quartus_report_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_report_timing_cmd{
    .name = "report_timing",
    .forms = eda_quartus_report_timing_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2152,
};

// report_ucp
inline constexpr FormDesc eda_quartus_report_ucp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_report_ucp_cmd{
    .name = "report_ucp",
    .forms = eda_quartus_report_ucp_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2153,
};

// save_report
inline constexpr FormDesc eda_quartus_save_report_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_save_report_cmd{
    .name = "save_report",
    .forms = eda_quartus_save_report_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2154,
};

// set_global_assignment
inline constexpr FormDesc eda_quartus_set_global_assignment_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_set_global_assignment_cmd{
    .name = "set_global_assignment",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_quartus_set_global_assignment_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2155,
};

// set_instance_assignment
inline constexpr FormDesc eda_quartus_set_instance_assignment_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_set_instance_assignment_cmd{
    .name = "set_instance_assignment",
    .forms = eda_quartus_set_instance_assignment_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2156,
};

// set_io_assignment
inline constexpr FormDesc eda_quartus_set_io_assignment_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_set_io_assignment_cmd{
    .name = "set_io_assignment",
    .forms = eda_quartus_set_io_assignment_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2157,
};

// set_location_assignment
inline constexpr FormDesc eda_quartus_set_location_assignment_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_set_location_assignment_cmd{
    .name = "set_location_assignment",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_quartus_set_location_assignment_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2158,
};

// set_parameter
inline constexpr FormDesc eda_quartus_set_parameter_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_set_parameter_cmd{
    .name = "set_parameter",
    .forms = eda_quartus_set_parameter_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2159,
};

// update_timing_netlist
inline constexpr FormDesc eda_quartus_update_timing_netlist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_quartus_update_timing_netlist_cmd{
    .name = "update_timing_netlist",
    .forms = eda_quartus_update_timing_netlist_forms,
    .dialects = DialectFlags::EDA_QUARTUS,
    .hover_index = 2160,
};
