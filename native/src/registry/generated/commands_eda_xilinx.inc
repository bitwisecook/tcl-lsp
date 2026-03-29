// ── eda_xilinx commands ──

// apply_bd_automation
inline constexpr FormDesc eda_xilinx_apply_bd_automation_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_apply_bd_automation_cmd{
    .name = "apply_bd_automation",
    .forms = eda_xilinx_apply_bd_automation_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2049,
};

// close_hw_manager
inline constexpr FormDesc eda_xilinx_close_hw_manager_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_close_hw_manager_cmd{
    .name = "close_hw_manager",
    .forms = eda_xilinx_close_hw_manager_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2050,
};

// close_project
inline constexpr FormDesc eda_xilinx_close_project_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_close_project_cmd{
    .name = "close_project",
    .forms = eda_xilinx_close_project_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2051,
};

// close_sim
inline constexpr FormDesc eda_xilinx_close_sim_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_close_sim_cmd{
    .name = "close_sim",
    .forms = eda_xilinx_close_sim_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2052,
};

// config_ip_cache
inline constexpr FormDesc eda_xilinx_config_ip_cache_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_config_ip_cache_cmd{
    .name = "config_ip_cache",
    .forms = eda_xilinx_config_ip_cache_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2053,
};

// connect_bd_intf_net
inline constexpr FormDesc eda_xilinx_connect_bd_intf_net_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_connect_bd_intf_net_cmd{
    .name = "connect_bd_intf_net",
    .forms = eda_xilinx_connect_bd_intf_net_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2054,
};

// connect_bd_net
inline constexpr FormDesc eda_xilinx_connect_bd_net_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_connect_bd_net_cmd{
    .name = "connect_bd_net",
    .forms = eda_xilinx_connect_bd_net_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2055,
};

// connect_hw_server
inline constexpr FormDesc eda_xilinx_connect_hw_server_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_connect_hw_server_cmd{
    .name = "connect_hw_server",
    .forms = eda_xilinx_connect_hw_server_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2056,
};

// create_bd_cell
inline constexpr FormDesc eda_xilinx_create_bd_cell_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_create_bd_cell_cmd{
    .name = "create_bd_cell",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_create_bd_cell_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2057,
};

// create_bd_design
inline constexpr FormDesc eda_xilinx_create_bd_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_create_bd_design_cmd{
    .name = "create_bd_design",
    .arity = {1, 1},
    .forms = eda_xilinx_create_bd_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2058,
};

// create_bd_intf_port
inline constexpr FormDesc eda_xilinx_create_bd_intf_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_create_bd_intf_port_cmd{
    .name = "create_bd_intf_port",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_create_bd_intf_port_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2059,
};

// create_bd_port
inline constexpr FormDesc eda_xilinx_create_bd_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_create_bd_port_cmd{
    .name = "create_bd_port",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_create_bd_port_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2060,
};

// create_ip
inline constexpr FormDesc eda_xilinx_create_ip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_create_ip_cmd{
    .name = "create_ip",
    .forms = eda_xilinx_create_ip_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2061,
};

// create_project
inline constexpr FormDesc eda_xilinx_create_project_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_create_project_cmd{
    .name = "create_project",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_create_project_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2062,
};

// create_run
inline constexpr FormDesc eda_xilinx_create_run_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_create_run_cmd{
    .name = "create_run",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_create_run_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2063,
};

// current_project
inline constexpr FormDesc eda_xilinx_current_project_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_current_project_cmd{
    .name = "current_project",
    .forms = eda_xilinx_current_project_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2064,
};

// generate_target
inline constexpr FormDesc eda_xilinx_generate_target_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_generate_target_cmd{
    .name = "generate_target",
    .forms = eda_xilinx_generate_target_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2065,
};

// get_ips
inline constexpr FormDesc eda_xilinx_get_ips_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_get_ips_cmd{
    .name = "get_ips",
    .forms = eda_xilinx_get_ips_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2066,
};

// get_projects
inline constexpr FormDesc eda_xilinx_get_projects_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_get_projects_cmd{
    .name = "get_projects",
    .forms = eda_xilinx_get_projects_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2067,
};

// get_property
inline constexpr FormDesc eda_xilinx_get_property_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_get_property_cmd{
    .name = "get_property",
    .arity = {2, 2},
    .forms = eda_xilinx_get_property_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2068,
};

// get_runs
inline constexpr FormDesc eda_xilinx_get_runs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_get_runs_cmd{
    .name = "get_runs",
    .forms = eda_xilinx_get_runs_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2069,
};

// import_ip
inline constexpr FormDesc eda_xilinx_import_ip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_import_ip_cmd{
    .name = "import_ip",
    .forms = eda_xilinx_import_ip_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2070,
};

// ipx::add_bus_interface
inline constexpr FormDesc eda_xilinx_ipx__add_bus_interface_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_ipx__add_bus_interface_cmd{
    .name = "ipx::add_bus_interface",
    .forms = eda_xilinx_ipx__add_bus_interface_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2071,
};

// ipx::package_project
inline constexpr FormDesc eda_xilinx_ipx__package_project_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_ipx__package_project_cmd{
    .name = "ipx::package_project",
    .forms = eda_xilinx_ipx__package_project_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2072,
};

// launch_runs
inline constexpr FormDesc eda_xilinx_launch_runs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_launch_runs_cmd{
    .name = "launch_runs",
    .forms = eda_xilinx_launch_runs_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2073,
};

// launch_simulation
inline constexpr FormDesc eda_xilinx_launch_simulation_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_launch_simulation_cmd{
    .name = "launch_simulation",
    .forms = eda_xilinx_launch_simulation_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2074,
};

// open_bd_design
inline constexpr FormDesc eda_xilinx_open_bd_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_open_bd_design_cmd{
    .name = "open_bd_design",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_open_bd_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2075,
};

// open_checkpoint
inline constexpr FormDesc eda_xilinx_open_checkpoint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_open_checkpoint_cmd{
    .name = "open_checkpoint",
    .arity = {1, 1},
    .forms = eda_xilinx_open_checkpoint_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2076,
};

// open_hw_manager
inline constexpr FormDesc eda_xilinx_open_hw_manager_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_open_hw_manager_cmd{
    .name = "open_hw_manager",
    .forms = eda_xilinx_open_hw_manager_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2077,
};

// open_hw_target
inline constexpr FormDesc eda_xilinx_open_hw_target_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_open_hw_target_cmd{
    .name = "open_hw_target",
    .forms = eda_xilinx_open_hw_target_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2078,
};

// open_project
inline constexpr FormDesc eda_xilinx_open_project_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_open_project_cmd{
    .name = "open_project",
    .arity = {1, 1},
    .forms = eda_xilinx_open_project_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2079,
};

// open_run
inline constexpr FormDesc eda_xilinx_open_run_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_open_run_cmd{
    .name = "open_run",
    .arity = {1, 1},
    .forms = eda_xilinx_open_run_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2080,
};

// opt_design
inline constexpr FormDesc eda_xilinx_opt_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_opt_design_cmd{
    .name = "opt_design",
    .forms = eda_xilinx_opt_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2081,
};

// phys_opt_design
inline constexpr FormDesc eda_xilinx_phys_opt_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_phys_opt_design_cmd{
    .name = "phys_opt_design",
    .forms = eda_xilinx_phys_opt_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2082,
};

// place_design
inline constexpr FormDesc eda_xilinx_place_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_place_design_cmd{
    .name = "place_design",
    .forms = eda_xilinx_place_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2083,
};

// program_hw_devices
inline constexpr FormDesc eda_xilinx_program_hw_devices_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_program_hw_devices_cmd{
    .name = "program_hw_devices",
    .forms = eda_xilinx_program_hw_devices_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2084,
};

// read_checkpoint
inline constexpr FormDesc eda_xilinx_read_checkpoint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_read_checkpoint_cmd{
    .name = "read_checkpoint",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_read_checkpoint_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2085,
};

// read_edif
inline constexpr FormDesc eda_xilinx_read_edif_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_read_edif_cmd{
    .name = "read_edif",
    .arity = {1, 1},
    .forms = eda_xilinx_read_edif_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2086,
};

// read_ip
inline constexpr FormDesc eda_xilinx_read_ip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_read_ip_cmd{
    .name = "read_ip",
    .forms = eda_xilinx_read_ip_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2087,
};

// read_verilog
inline constexpr FormDesc eda_xilinx_read_verilog_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_read_verilog_cmd{
    .name = "read_verilog",
    .forms = eda_xilinx_read_verilog_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2088,
};

// read_vhdl
inline constexpr FormDesc eda_xilinx_read_vhdl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_read_vhdl_cmd{
    .name = "read_vhdl",
    .forms = eda_xilinx_read_vhdl_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2089,
};

// read_xdc
inline constexpr FormDesc eda_xilinx_read_xdc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_read_xdc_cmd{
    .name = "read_xdc",
    .forms = eda_xilinx_read_xdc_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2090,
};

// report_clock_networks
inline constexpr FormDesc eda_xilinx_report_clock_networks_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_clock_networks_cmd{
    .name = "report_clock_networks",
    .forms = eda_xilinx_report_clock_networks_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2091,
};

// report_clock_utilization
inline constexpr FormDesc eda_xilinx_report_clock_utilization_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_clock_utilization_cmd{
    .name = "report_clock_utilization",
    .forms = eda_xilinx_report_clock_utilization_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2092,
};

// report_design_analysis
inline constexpr FormDesc eda_xilinx_report_design_analysis_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_design_analysis_cmd{
    .name = "report_design_analysis",
    .forms = eda_xilinx_report_design_analysis_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2093,
};

// report_drc
inline constexpr FormDesc eda_xilinx_report_drc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_drc_cmd{
    .name = "report_drc",
    .forms = eda_xilinx_report_drc_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2094,
};

// report_io
inline constexpr FormDesc eda_xilinx_report_io_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_io_cmd{
    .name = "report_io",
    .forms = eda_xilinx_report_io_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2095,
};

// report_methodology
inline constexpr FormDesc eda_xilinx_report_methodology_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_methodology_cmd{
    .name = "report_methodology",
    .forms = eda_xilinx_report_methodology_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2096,
};

// report_power
inline constexpr FormDesc eda_xilinx_report_power_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_power_cmd{
    .name = "report_power",
    .forms = eda_xilinx_report_power_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2097,
};

// report_route_status
inline constexpr FormDesc eda_xilinx_report_route_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_route_status_cmd{
    .name = "report_route_status",
    .forms = eda_xilinx_report_route_status_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2098,
};

// report_timing
inline constexpr FormDesc eda_xilinx_report_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_timing_cmd{
    .name = "report_timing",
    .forms = eda_xilinx_report_timing_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2099,
};

// report_timing_summary
inline constexpr FormDesc eda_xilinx_report_timing_summary_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_timing_summary_cmd{
    .name = "report_timing_summary",
    .forms = eda_xilinx_report_timing_summary_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2100,
};

// report_utilization
inline constexpr FormDesc eda_xilinx_report_utilization_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_report_utilization_cmd{
    .name = "report_utilization",
    .forms = eda_xilinx_report_utilization_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2101,
};

// reset_run
inline constexpr FormDesc eda_xilinx_reset_run_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_reset_run_cmd{
    .name = "reset_run",
    .arity = {1, 1},
    .forms = eda_xilinx_reset_run_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2102,
};

// route_design
inline constexpr FormDesc eda_xilinx_route_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_route_design_cmd{
    .name = "route_design",
    .forms = eda_xilinx_route_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2103,
};

// save_bd_design
inline constexpr FormDesc eda_xilinx_save_bd_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_save_bd_design_cmd{
    .name = "save_bd_design",
    .forms = eda_xilinx_save_bd_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2104,
};

// save_project_as
inline constexpr FormDesc eda_xilinx_save_project_as_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_save_project_as_cmd{
    .name = "save_project_as",
    .forms = eda_xilinx_save_project_as_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2105,
};

// set_property
inline constexpr FormDesc eda_xilinx_set_property_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_set_property_cmd{
    .name = "set_property",
    .arity = {3, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_set_property_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2106,
};

// synth_design
inline constexpr FormDesc eda_xilinx_synth_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_synth_design_cmd{
    .name = "synth_design",
    .forms = eda_xilinx_synth_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2107,
};

// upgrade_ip
inline constexpr FormDesc eda_xilinx_upgrade_ip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_upgrade_ip_cmd{
    .name = "upgrade_ip",
    .forms = eda_xilinx_upgrade_ip_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2108,
};

// validate_bd_design
inline constexpr FormDesc eda_xilinx_validate_bd_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_validate_bd_design_cmd{
    .name = "validate_bd_design",
    .forms = eda_xilinx_validate_bd_design_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2109,
};

// wait_on_run
inline constexpr FormDesc eda_xilinx_wait_on_run_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_wait_on_run_cmd{
    .name = "wait_on_run",
    .arity = {1, 1},
    .forms = eda_xilinx_wait_on_run_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2110,
};

// write_bitstream
inline constexpr FormDesc eda_xilinx_write_bitstream_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_write_bitstream_cmd{
    .name = "write_bitstream",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_write_bitstream_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2111,
};

// write_checkpoint
inline constexpr FormDesc eda_xilinx_write_checkpoint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_xilinx_write_checkpoint_cmd{
    .name = "write_checkpoint",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_xilinx_write_checkpoint_forms,
    .dialects = DialectFlags::EDA_XILINX,
    .hover_index = 2112,
};
