// ── eda_sdc commands ──

// all_clocks
inline constexpr FormDesc eda_sdc_all_clocks_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_all_clocks_cmd{
    .name = "all_clocks",
    .forms = eda_sdc_all_clocks_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2210,
};

// all_fanin
inline constexpr FormDesc eda_sdc_all_fanin_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_all_fanin_cmd{
    .name = "all_fanin",
    .forms = eda_sdc_all_fanin_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2211,
};

// all_fanout
inline constexpr FormDesc eda_sdc_all_fanout_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_all_fanout_cmd{
    .name = "all_fanout",
    .forms = eda_sdc_all_fanout_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2212,
};

// all_inputs
inline constexpr FormDesc eda_sdc_all_inputs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_all_inputs_cmd{
    .name = "all_inputs",
    .forms = eda_sdc_all_inputs_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2213,
};

// all_outputs
inline constexpr FormDesc eda_sdc_all_outputs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_all_outputs_cmd{
    .name = "all_outputs",
    .forms = eda_sdc_all_outputs_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2214,
};

// all_registers
inline constexpr FormDesc eda_sdc_all_registers_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_all_registers_cmd{
    .name = "all_registers",
    .forms = eda_sdc_all_registers_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2215,
};

// append_to_collection
inline constexpr ArgPattern eda_sdc_append_to_collection_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr FormDesc eda_sdc_append_to_collection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_append_to_collection_cmd{
    .name = "append_to_collection",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = eda_sdc_append_to_collection_args,
    .forms = eda_sdc_append_to_collection_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2216,
};

// check_timing
inline constexpr FormDesc eda_sdc_check_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_check_timing_cmd{
    .name = "check_timing",
    .forms = eda_sdc_check_timing_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2217,
};

// create_clock
inline constexpr FormDesc eda_sdc_create_clock_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_create_clock_cmd{
    .name = "create_clock",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_sdc_create_clock_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2218,
};

// create_generated_clock
inline constexpr FormDesc eda_sdc_create_generated_clock_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_create_generated_clock_cmd{
    .name = "create_generated_clock",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_sdc_create_generated_clock_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2219,
};

// current_design
inline constexpr FormDesc eda_sdc_current_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_current_design_cmd{
    .name = "current_design",
    .forms = eda_sdc_current_design_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2220,
};

// define_proc_attributes
inline constexpr FormDesc eda_sdc_define_proc_attributes_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_define_proc_attributes_cmd{
    .name = "define_proc_attributes",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_sdc_define_proc_attributes_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2221,
};

// filter_collection
inline constexpr FormDesc eda_sdc_filter_collection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_filter_collection_cmd{
    .name = "filter_collection",
    .arity = {2, 2},
    .forms = eda_sdc_filter_collection_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2222,
};

// foreach_in_collection
inline constexpr ArgPattern eda_sdc_foreach_in_collection_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
    {.kind = FIXED, .role = BODY, .index = 2},
};

inline constexpr FormDesc eda_sdc_foreach_in_collection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_foreach_in_collection_cmd{
    .name = "foreach_in_collection",
    .arity = {3, 3},
    .arg_patterns = eda_sdc_foreach_in_collection_args,
    .forms = eda_sdc_foreach_in_collection_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .is_control_flow = true,
    .never_inline_body = true,
    .has_loop_body = true,
    .loop_list_header = true,
    .hover_index = 2223,
};

// get_cells
inline constexpr FormDesc eda_sdc_get_cells_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_cells_cmd{
    .name = "get_cells",
    .forms = eda_sdc_get_cells_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2224,
};

// get_clocks
inline constexpr FormDesc eda_sdc_get_clocks_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_clocks_cmd{
    .name = "get_clocks",
    .forms = eda_sdc_get_clocks_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2225,
};

// get_lib_cells
inline constexpr FormDesc eda_sdc_get_lib_cells_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_lib_cells_cmd{
    .name = "get_lib_cells",
    .forms = eda_sdc_get_lib_cells_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2226,
};

// get_lib_pins
inline constexpr FormDesc eda_sdc_get_lib_pins_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_lib_pins_cmd{
    .name = "get_lib_pins",
    .forms = eda_sdc_get_lib_pins_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2227,
};

// get_libs
inline constexpr FormDesc eda_sdc_get_libs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_libs_cmd{
    .name = "get_libs",
    .forms = eda_sdc_get_libs_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2228,
};

// get_nets
inline constexpr FormDesc eda_sdc_get_nets_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_nets_cmd{
    .name = "get_nets",
    .forms = eda_sdc_get_nets_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2229,
};

// get_object_name
inline constexpr FormDesc eda_sdc_get_object_name_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_object_name_cmd{
    .name = "get_object_name",
    .arity = {1, 1},
    .forms = eda_sdc_get_object_name_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2230,
};

// get_pins
inline constexpr FormDesc eda_sdc_get_pins_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_pins_cmd{
    .name = "get_pins",
    .forms = eda_sdc_get_pins_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2231,
};

// get_ports
inline constexpr FormDesc eda_sdc_get_ports_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_get_ports_cmd{
    .name = "get_ports",
    .forms = eda_sdc_get_ports_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2232,
};

// group_path
inline constexpr FormDesc eda_sdc_group_path_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_group_path_cmd{
    .name = "group_path",
    .forms = eda_sdc_group_path_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2233,
};

// link_design
inline constexpr FormDesc eda_sdc_link_design_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_link_design_cmd{
    .name = "link_design",
    .forms = eda_sdc_link_design_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2234,
};

// remove_from_collection
inline constexpr ArgPattern eda_sdc_remove_from_collection_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr FormDesc eda_sdc_remove_from_collection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_remove_from_collection_cmd{
    .name = "remove_from_collection",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = eda_sdc_remove_from_collection_args,
    .forms = eda_sdc_remove_from_collection_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2235,
};

// report_area
inline constexpr FormDesc eda_sdc_report_area_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_report_area_cmd{
    .name = "report_area",
    .forms = eda_sdc_report_area_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2236,
};

// report_clock
inline constexpr FormDesc eda_sdc_report_clock_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_report_clock_cmd{
    .name = "report_clock",
    .forms = eda_sdc_report_clock_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2237,
};

// report_clock_timing
inline constexpr FormDesc eda_sdc_report_clock_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_report_clock_timing_cmd{
    .name = "report_clock_timing",
    .forms = eda_sdc_report_clock_timing_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2238,
};

// report_constraint
inline constexpr FormDesc eda_sdc_report_constraint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_report_constraint_cmd{
    .name = "report_constraint",
    .forms = eda_sdc_report_constraint_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2239,
};

// report_power
inline constexpr FormDesc eda_sdc_report_power_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_report_power_cmd{
    .name = "report_power",
    .forms = eda_sdc_report_power_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2240,
};

// report_timing
inline constexpr FormDesc eda_sdc_report_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_report_timing_cmd{
    .name = "report_timing",
    .forms = eda_sdc_report_timing_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2241,
};

// set_case_analysis
inline constexpr FormDesc eda_sdc_set_case_analysis_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_case_analysis_cmd{
    .name = "set_case_analysis",
    .forms = eda_sdc_set_case_analysis_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2242,
};

// set_clock_groups
inline constexpr FormDesc eda_sdc_set_clock_groups_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_clock_groups_cmd{
    .name = "set_clock_groups",
    .forms = eda_sdc_set_clock_groups_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2243,
};

// set_clock_latency
inline constexpr FormDesc eda_sdc_set_clock_latency_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_clock_latency_cmd{
    .name = "set_clock_latency",
    .forms = eda_sdc_set_clock_latency_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2244,
};

// set_clock_transition
inline constexpr FormDesc eda_sdc_set_clock_transition_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_clock_transition_cmd{
    .name = "set_clock_transition",
    .forms = eda_sdc_set_clock_transition_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2245,
};

// set_clock_uncertainty
inline constexpr FormDesc eda_sdc_set_clock_uncertainty_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_clock_uncertainty_cmd{
    .name = "set_clock_uncertainty",
    .forms = eda_sdc_set_clock_uncertainty_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2246,
};

// set_disable_timing
inline constexpr FormDesc eda_sdc_set_disable_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_disable_timing_cmd{
    .name = "set_disable_timing",
    .forms = eda_sdc_set_disable_timing_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2247,
};

// set_dont_touch
inline constexpr FormDesc eda_sdc_set_dont_touch_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_dont_touch_cmd{
    .name = "set_dont_touch",
    .forms = eda_sdc_set_dont_touch_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2248,
};

// set_dont_use
inline constexpr FormDesc eda_sdc_set_dont_use_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_dont_use_cmd{
    .name = "set_dont_use",
    .forms = eda_sdc_set_dont_use_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2249,
};

// set_driving_cell
inline constexpr FormDesc eda_sdc_set_driving_cell_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_driving_cell_cmd{
    .name = "set_driving_cell",
    .forms = eda_sdc_set_driving_cell_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2250,
};

// set_false_path
inline constexpr FormDesc eda_sdc_set_false_path_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_false_path_cmd{
    .name = "set_false_path",
    .forms = eda_sdc_set_false_path_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2251,
};

// set_ideal_latency
inline constexpr FormDesc eda_sdc_set_ideal_latency_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_ideal_latency_cmd{
    .name = "set_ideal_latency",
    .forms = eda_sdc_set_ideal_latency_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2252,
};

// set_ideal_network
inline constexpr FormDesc eda_sdc_set_ideal_network_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_ideal_network_cmd{
    .name = "set_ideal_network",
    .forms = eda_sdc_set_ideal_network_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2253,
};

// set_input_delay
inline constexpr FormDesc eda_sdc_set_input_delay_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_input_delay_cmd{
    .name = "set_input_delay",
    .forms = eda_sdc_set_input_delay_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2254,
};

// set_input_transition
inline constexpr FormDesc eda_sdc_set_input_transition_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_input_transition_cmd{
    .name = "set_input_transition",
    .forms = eda_sdc_set_input_transition_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2255,
};

// set_load
inline constexpr FormDesc eda_sdc_set_load_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_load_cmd{
    .name = "set_load",
    .forms = eda_sdc_set_load_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2256,
};

// set_max_area
inline constexpr FormDesc eda_sdc_set_max_area_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_max_area_cmd{
    .name = "set_max_area",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_sdc_set_max_area_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2257,
};

// set_max_capacitance
inline constexpr FormDesc eda_sdc_set_max_capacitance_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_max_capacitance_cmd{
    .name = "set_max_capacitance",
    .forms = eda_sdc_set_max_capacitance_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2258,
};

// set_max_delay
inline constexpr FormDesc eda_sdc_set_max_delay_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_max_delay_cmd{
    .name = "set_max_delay",
    .forms = eda_sdc_set_max_delay_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2259,
};

// set_max_fanout
inline constexpr FormDesc eda_sdc_set_max_fanout_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_max_fanout_cmd{
    .name = "set_max_fanout",
    .forms = eda_sdc_set_max_fanout_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2260,
};

// set_max_transition
inline constexpr FormDesc eda_sdc_set_max_transition_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_max_transition_cmd{
    .name = "set_max_transition",
    .forms = eda_sdc_set_max_transition_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2261,
};

// set_min_delay
inline constexpr FormDesc eda_sdc_set_min_delay_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_min_delay_cmd{
    .name = "set_min_delay",
    .forms = eda_sdc_set_min_delay_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2262,
};

// set_multicycle_path
inline constexpr FormDesc eda_sdc_set_multicycle_path_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_multicycle_path_cmd{
    .name = "set_multicycle_path",
    .forms = eda_sdc_set_multicycle_path_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2263,
};

// set_output_delay
inline constexpr FormDesc eda_sdc_set_output_delay_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_output_delay_cmd{
    .name = "set_output_delay",
    .forms = eda_sdc_set_output_delay_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2264,
};

// set_propagated_clock
inline constexpr FormDesc eda_sdc_set_propagated_clock_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_propagated_clock_cmd{
    .name = "set_propagated_clock",
    .forms = eda_sdc_set_propagated_clock_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2265,
};

// set_size_only
inline constexpr FormDesc eda_sdc_set_size_only_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_size_only_cmd{
    .name = "set_size_only",
    .forms = eda_sdc_set_size_only_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2266,
};

// set_units
inline constexpr FormDesc eda_sdc_set_units_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_units_cmd{
    .name = "set_units",
    .forms = eda_sdc_set_units_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2267,
};

// set_wire_load_mode
inline constexpr FormDesc eda_sdc_set_wire_load_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_wire_load_mode_cmd{
    .name = "set_wire_load_mode",
    .arity = {1, 1},
    .forms = eda_sdc_set_wire_load_mode_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2268,
};

// set_wire_load_model
inline constexpr FormDesc eda_sdc_set_wire_load_model_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_set_wire_load_model_cmd{
    .name = "set_wire_load_model",
    .forms = eda_sdc_set_wire_load_model_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2269,
};

// sizeof_collection
inline constexpr FormDesc eda_sdc_sizeof_collection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_sdc_sizeof_collection_cmd{
    .name = "sizeof_collection",
    .arity = {1, 1},
    .forms = eda_sdc_sizeof_collection_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_XILINX,
    .hover_index = 2270,
};
