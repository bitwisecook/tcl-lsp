// ── eda_mentor commands ──

// add_list
inline constexpr FormDesc eda_mentor_add_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_add_list_cmd{
    .name = "add_list",
    .forms = eda_mentor_add_list_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2161,
};

// add_log
inline constexpr FormDesc eda_mentor_add_log_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_add_log_cmd{
    .name = "add_log",
    .forms = eda_mentor_add_log_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2162,
};

// add_wave
inline constexpr FormDesc eda_mentor_add_wave_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_add_wave_cmd{
    .name = "add_wave",
    .forms = eda_mentor_add_wave_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2163,
};

// bc
inline constexpr FormDesc eda_mentor_bc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_bc_cmd{
    .name = "bc",
    .forms = eda_mentor_bc_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2164,
};

// bd
inline constexpr FormDesc eda_mentor_bd_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_bd_cmd{
    .name = "bd",
    .forms = eda_mentor_bd_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2165,
};

// be
inline constexpr FormDesc eda_mentor_be_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_be_cmd{
    .name = "be",
    .forms = eda_mentor_be_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2166,
};

// bl
inline constexpr FormDesc eda_mentor_bl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_bl_cmd{
    .name = "bl",
    .forms = eda_mentor_bl_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2167,
};

// bp
inline constexpr FormDesc eda_mentor_bp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_bp_cmd{
    .name = "bp",
    .forms = eda_mentor_bp_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2168,
};

// calibre
inline constexpr FormDesc eda_mentor_calibre_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_calibre_cmd{
    .name = "calibre",
    .forms = eda_mentor_calibre_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2169,
};

// calibre_drc
inline constexpr FormDesc eda_mentor_calibre_drc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_calibre_drc_cmd{
    .name = "calibre_drc",
    .forms = eda_mentor_calibre_drc_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2170,
};

// calibre_lvs
inline constexpr FormDesc eda_mentor_calibre_lvs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_calibre_lvs_cmd{
    .name = "calibre_lvs",
    .forms = eda_mentor_calibre_lvs_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2171,
};

// calibre_pex
inline constexpr FormDesc eda_mentor_calibre_pex_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_calibre_pex_cmd{
    .name = "calibre_pex",
    .forms = eda_mentor_calibre_pex_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2172,
};

// change
inline constexpr FormDesc eda_mentor_change_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_change_cmd{
    .name = "change",
    .forms = eda_mentor_change_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2173,
};

// coverage
inline constexpr FormDesc eda_mentor_coverage_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_coverage_cmd{
    .name = "coverage",
    .forms = eda_mentor_coverage_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2174,
};

// describe
inline constexpr FormDesc eda_mentor_describe_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_describe_cmd{
    .name = "describe",
    .arity = {1, 1},
    .forms = eda_mentor_describe_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2175,
};

// drivers
inline constexpr FormDesc eda_mentor_drivers_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_drivers_cmd{
    .name = "drivers",
    .arity = {1, 1},
    .forms = eda_mentor_drivers_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2176,
};

// examine
inline constexpr FormDesc eda_mentor_examine_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_examine_cmd{
    .name = "examine",
    .forms = eda_mentor_examine_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2177,
};

// find
inline constexpr FormDesc eda_mentor_find_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_find_cmd{
    .name = "find",
    .forms = eda_mentor_find_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2178,
};

// force
inline constexpr FormDesc eda_mentor_force_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_force_cmd{
    .name = "force",
    .forms = eda_mentor_force_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2179,
};

// formal_analyze
inline constexpr FormDesc eda_mentor_formal_analyze_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_formal_analyze_cmd{
    .name = "formal_analyze",
    .forms = eda_mentor_formal_analyze_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2180,
};

// formal_compile
inline constexpr FormDesc eda_mentor_formal_compile_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_formal_compile_cmd{
    .name = "formal_compile",
    .forms = eda_mentor_formal_compile_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2181,
};

// formal_verify
inline constexpr FormDesc eda_mentor_formal_verify_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_formal_verify_cmd{
    .name = "formal_verify",
    .forms = eda_mentor_formal_verify_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2182,
};

// init_signal_driver
inline constexpr FormDesc eda_mentor_init_signal_driver_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_init_signal_driver_cmd{
    .name = "init_signal_driver",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_mentor_init_signal_driver_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2183,
};

// init_signal_spy
inline constexpr FormDesc eda_mentor_init_signal_spy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_init_signal_spy_cmd{
    .name = "init_signal_spy",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_mentor_init_signal_spy_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2184,
};

// onbreak
inline constexpr FormDesc eda_mentor_onbreak_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_onbreak_cmd{
    .name = "onbreak",
    .forms = eda_mentor_onbreak_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2185,
};

// qrun
inline constexpr FormDesc eda_mentor_qrun_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_qrun_cmd{
    .name = "qrun",
    .forms = eda_mentor_qrun_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2186,
};

// qverilog
inline constexpr FormDesc eda_mentor_qverilog_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_qverilog_cmd{
    .name = "qverilog",
    .forms = eda_mentor_qverilog_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2187,
};

// qvhdl
inline constexpr FormDesc eda_mentor_qvhdl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_qvhdl_cmd{
    .name = "qvhdl",
    .forms = eda_mentor_qvhdl_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2188,
};

// qwave
inline constexpr FormDesc eda_mentor_qwave_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_qwave_cmd{
    .name = "qwave",
    .forms = eda_mentor_qwave_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2189,
};

// readers
inline constexpr FormDesc eda_mentor_readers_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_readers_cmd{
    .name = "readers",
    .arity = {1, 1},
    .forms = eda_mentor_readers_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2190,
};

// release
inline constexpr FormDesc eda_mentor_release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_release_cmd{
    .name = "release",
    .forms = eda_mentor_release_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2191,
};

// restart
inline constexpr FormDesc eda_mentor_restart_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_restart_cmd{
    .name = "restart",
    .forms = eda_mentor_restart_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2192,
};

// resume
inline constexpr FormDesc eda_mentor_resume_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_resume_cmd{
    .name = "resume",
    .forms = eda_mentor_resume_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2193,
};

// run
inline constexpr FormDesc eda_mentor_run_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_run_cmd{
    .name = "run",
    .forms = eda_mentor_run_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2194,
};

// signal_force
inline constexpr FormDesc eda_mentor_signal_force_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_signal_force_cmd{
    .name = "signal_force",
    .forms = eda_mentor_signal_force_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2195,
};

// signal_release
inline constexpr FormDesc eda_mentor_signal_release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_signal_release_cmd{
    .name = "signal_release",
    .forms = eda_mentor_signal_release_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2196,
};

// toggle
inline constexpr FormDesc eda_mentor_toggle_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_toggle_cmd{
    .name = "toggle",
    .forms = eda_mentor_toggle_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2197,
};

// transcript
inline constexpr FormDesc eda_mentor_transcript_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_transcript_cmd{
    .name = "transcript",
    .forms = eda_mentor_transcript_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2198,
};

// vcom
inline constexpr FormDesc eda_mentor_vcom_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_vcom_cmd{
    .name = "vcom",
    .forms = eda_mentor_vcom_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2199,
};

// vcover
inline constexpr FormDesc eda_mentor_vcover_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_vcover_cmd{
    .name = "vcover",
    .forms = eda_mentor_vcover_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2200,
};

// vdel
inline constexpr FormDesc eda_mentor_vdel_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_vdel_cmd{
    .name = "vdel",
    .forms = eda_mentor_vdel_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2201,
};

// virtual
inline constexpr FormDesc eda_mentor_virtual_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_virtual_cmd{
    .name = "virtual",
    .forms = eda_mentor_virtual_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2202,
};

// vlib
inline constexpr FormDesc eda_mentor_vlib_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_vlib_cmd{
    .name = "vlib",
    .arity = {1, 1},
    .forms = eda_mentor_vlib_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2203,
};

// vlog
inline constexpr FormDesc eda_mentor_vlog_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_vlog_cmd{
    .name = "vlog",
    .forms = eda_mentor_vlog_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2204,
};

// vmap
inline constexpr FormDesc eda_mentor_vmap_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_vmap_cmd{
    .name = "vmap",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = eda_mentor_vmap_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2205,
};

// vopt
inline constexpr FormDesc eda_mentor_vopt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_vopt_cmd{
    .name = "vopt",
    .forms = eda_mentor_vopt_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2206,
};

// vsim
inline constexpr FormDesc eda_mentor_vsim_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_vsim_cmd{
    .name = "vsim",
    .forms = eda_mentor_vsim_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2207,
};

// wave
inline constexpr FormDesc eda_mentor_wave_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_wave_cmd{
    .name = "wave",
    .forms = eda_mentor_wave_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .hover_index = 2208,
};

// when
inline constexpr FormDesc eda_mentor_when_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc eda_mentor_when_cmd{
    .name = "when",
    .layout_resolver = LayoutResolver::WHEN,
    .forms = eda_mentor_when_forms,
    .dialects = DialectFlags::EDA_MENTOR,
    .top_level_only = true,
    .hover_index = 2209,
};
