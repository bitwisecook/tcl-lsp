// ── tcl commands ──

// after
inline constexpr FormDesc tcl_after_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_after_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_after_cmd{
    .name = "after",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_after_forms,
    .side_effect_hints = tcl_after_effects,
    .hover_index = 0,
};

// append
inline constexpr ArgPattern tcl_append_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgTypeDesc tcl_append_types[] = {
    {.index = 0, .expected = TclType::STRING, .shimmers = true},
};

inline constexpr FormDesc tcl_append_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_append_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_append_cmd{
    .name = "append",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_append_args,
    .forms = tcl_append_forms,
    .assigns_variable_at = 0,
    .arg_types = tcl_append_types,
    .side_effect_hints = tcl_append_effects,
    .hover_index = 1,
};

// apply
inline constexpr ArgPattern tcl_apply_args[] = {
    {.kind = FIXED, .role = BODY, .index = 0},
};

inline constexpr FormDesc tcl_apply_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_apply_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_apply_cmd{
    .name = "apply",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_apply_args,
    .forms = tcl_apply_forms,
    .is_scope_barrier = true,
    .side_effect_hints = tcl_apply_effects,
    .hover_index = 2,
};

// array
inline constexpr FormDesc tcl_array_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_array_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true},
};

inline constexpr ArgPattern tcl_array_sub_anymore_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_donesearch_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_exists_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_get_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_names_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_nextelement_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_set_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_size_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_startsearch_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_statistics_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr ArgPattern tcl_array_sub_unset_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr SubCmdDesc tcl_array_subcmds[] = {
    {.name = "anymore", .arity = {2, 2}, .arg_patterns = tcl_array_sub_anymore_args, .return_type = TclType::BOOLEAN, .hover_index = 3},
    {.name = "donesearch", .arity = {2, 2}, .arg_patterns = tcl_array_sub_donesearch_args, .hover_index = 4},
    {.name = "exists", .arity = {1, 1}, .arg_patterns = tcl_array_sub_exists_args, .return_type = TclType::BOOLEAN, .hover_index = 5},
    {.name = "get", .arity = {1, 2}, .arg_patterns = tcl_array_sub_get_args, .return_type = TclType::LIST, .hover_index = 6},
    {.name = "names", .arity = {1, 3}, .arg_patterns = tcl_array_sub_names_args, .return_type = TclType::LIST, .hover_index = 7},
    {.name = "nextelement", .arity = {2, 2}, .arg_patterns = tcl_array_sub_nextelement_args, .hover_index = 8},
    {.name = "set", .arity = {2, 2}, .arg_patterns = tcl_array_sub_set_args, .mutator = true, .hover_index = 9},
    {.name = "size", .arity = {1, 1}, .arg_patterns = tcl_array_sub_size_args, .return_type = TclType::INT, .hover_index = 10},
    {.name = "startsearch", .arity = {1, 1}, .arg_patterns = tcl_array_sub_startsearch_args, .hover_index = 11},
    {.name = "statistics", .arity = {1, 1}, .arg_patterns = tcl_array_sub_statistics_args, .hover_index = 12},
    {.name = "unset", .arity = {1, 2}, .arg_patterns = tcl_array_sub_unset_args, .mutator = true, .hover_index = 13},
};

inline constexpr CommandDesc tcl_array_cmd{
    .name = "array",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_array_subcmds,
    .forms = tcl_array_forms,
    .assigns_variable_at = 1,
    .side_effect_hints = tcl_array_effects,
    .hover_index = 14,
};

// auto_execok
inline constexpr CommandDesc tcl_auto_execok_cmd{
    .name = "auto_execok",
    .arity = {1, 1},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 15,
};

// auto_import
inline constexpr CommandDesc tcl_auto_import_cmd{
    .name = "auto_import",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 16,
};

// auto_load
inline constexpr CommandDesc tcl_auto_load_cmd{
    .name = "auto_load",
    .arity = {1, 1},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 17,
};

// auto_mkindex
inline constexpr CommandDesc tcl_auto_mkindex_cmd{
    .name = "auto_mkindex",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 18,
};

// auto_mkindex_old
inline constexpr CommandDesc tcl_auto_mkindex_old_cmd{
    .name = "auto_mkindex_old",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 19,
};

// auto_qualify
inline constexpr CommandDesc tcl_auto_qualify_cmd{
    .name = "auto_qualify",
    .arity = {2, 2},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 20,
};

// auto_reset
inline constexpr CommandDesc tcl_auto_reset_cmd{
    .name = "auto_reset",
    .arity = {0, 0},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 21,
};

// bgerror
inline constexpr CommandDesc tcl_bgerror_cmd{
    .name = "bgerror",
    .arity = {1, 1},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 22,
};

// binary
inline constexpr FormDesc tcl_binary_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_binary_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr ArgPattern tcl_binary_sub_scan_args[] = {
    {.kind = TAIL, .role = VAR_NAME, .index = 2},
};

inline constexpr ArgTypeDesc tcl_binary_sub_decode_types[] = {
    {.index = 0, .expected = TclType::STRING},
};

inline constexpr ArgTypeDesc tcl_binary_sub_encode_types[] = {
    {.index = 0, .expected = TclType::STRING},
};

inline constexpr ArgTypeDesc tcl_binary_sub_format_types[] = {
    {.index = 0, .expected = TclType::STRING},
};

inline constexpr ArgTypeDesc tcl_binary_sub_scan_types[] = {
    {.index = 0, .expected = TclType::BYTEARRAY},
    {.index = 1, .expected = TclType::STRING},
};

inline constexpr SubCmdDesc tcl_binary_subcmds[] = {
    {.name = "decode", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_types = tcl_binary_sub_decode_types, .return_type = TclType::BYTEARRAY, .hover_index = 23},
    {.name = "encode", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_types = tcl_binary_sub_encode_types, .hover_index = 24},
    {.name = "format", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .arg_types = tcl_binary_sub_format_types, .return_type = TclType::BYTEARRAY, .hover_index = 25},
    {.name = "scan", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_binary_sub_scan_args, .arg_types = tcl_binary_sub_scan_types, .return_type = TclType::INT, .hover_index = 26},
};

inline constexpr CommandDesc tcl_binary_cmd{
    .name = "binary",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_binary_subcmds,
    .forms = tcl_binary_forms,
    .side_effect_hints = tcl_binary_effects,
    .hover_index = 27,
};

// break
inline constexpr FormDesc tcl_break_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_break_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_break_cmd{
    .name = "break",
    .arity = {0, 0},
    .forms = tcl_break_forms,
    .needs_start_cmd = true,
    .side_effect_hints = tcl_break_effects,
    .hover_index = 28,
};

// catch
inline constexpr ArgPattern tcl_catch_args[] = {
    {.kind = FIXED, .role = BODY, .index = 0},
    {.kind = FIXED, .role = VAR_NAME, .index = 1},
    {.kind = FIXED, .role = VAR_NAME, .index = 2},
};

inline constexpr FormDesc tcl_catch_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_catch_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_catch_cmd{
    .name = "catch",
    .arity = {1, 3},
    .arg_patterns = tcl_catch_args,
    .forms = tcl_catch_forms,
    .is_control_flow = true,
    .return_type = TclType::INT,
    .side_effect_hints = tcl_catch_effects,
    .hover_index = 29,
};

// cd
inline constexpr FormDesc tcl_cd_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_cd_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .writes = true},
};

inline constexpr CommandDesc tcl_cd_cmd{
    .name = "cd",
    .arity = {0, 1},
    .forms = tcl_cd_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_cd_effects,
    .hover_index = 30,
};

// chan
inline constexpr OptionDesc tcl_chan_opts[] = {
    {.name = "-blocking", .takes_value = true, .value_hint = "boolean", .detail = "Set blocking mode."},
    {.name = "-buffering", .takes_value = true, .value_hint = "mode", .detail = "Set buffering mode (full, line, none)."},
    {.name = "-buffersize", .takes_value = true, .value_hint = "size", .detail = "Set buffer size in bytes."},
    {.name = "-encoding", .takes_value = true, .value_hint = "encoding", .detail = "Set character encoding."},
    {.name = "-eofchar", .takes_value = true, .value_hint = "chars", .detail = "Set end-of-file character(s)."},
    {.name = "-profile", .takes_value = true, .value_hint = "profile", .detail = "Set encoding profile (strict, tcl8, replace)."},
    {.name = "-translation", .takes_value = true, .value_hint = "mode", .detail = "Set line-ending translation mode."},
};

inline constexpr FormDesc tcl_chan_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_chan_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .writes = true},
};

inline constexpr ArgPattern tcl_chan_sub_event_args[] = {
    {.kind = FIXED, .role = BODY, .index = 2},
};

inline constexpr ArgPattern tcl_chan_sub_gets_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 1},
};

inline constexpr SubCmdDesc tcl_chan_subcmds[] = {
    {.name = "blocked", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 31},
    {.name = "close", .arity = {1, 2}, .hover_index = 32},
    {.name = "configure", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 33},
    {.name = "copy", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::INT, .hover_index = 34},
    {.name = "create", .arity = {2, 2}, .hover_index = 35},
    {.name = "eof", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 36},
    {.name = "event", .arity = {2, 3}, .arg_patterns = tcl_chan_sub_event_args, .hover_index = 37},
    {.name = "flush", .arity = {1, 1}, .hover_index = 38},
    {.name = "gets", .arity = {1, 2}, .arg_patterns = tcl_chan_sub_gets_args, .hover_index = 39},
    {.name = "isbinary", .arity = {1, 1}, .hover_index = 40},
    {.name = "names", .arity = {0, 1}, .return_type = TclType::LIST, .hover_index = 41},
    {.name = "pending", .arity = {2, 2}, .return_type = TclType::INT, .hover_index = 42},
    {.name = "pipe", .arity = {0, 0}, .return_type = TclType::LIST, .hover_index = 43},
    {.name = "pop", .arity = {1, 1}, .hover_index = 44},
    {.name = "postevent", .arity = {2, 2}, .hover_index = 45},
    {.name = "push", .arity = {2, 2}, .hover_index = 46},
    {.name = "puts", .arity = {1, 3}, .hover_index = 47},
    {.name = "read", .arity = {1, 2}, .hover_index = 48},
    {.name = "seek", .arity = {2, 3}, .hover_index = 49},
    {.name = "tell", .arity = {1, 1}, .return_type = TclType::INT, .hover_index = 50},
    {.name = "truncate", .arity = {1, 2}, .hover_index = 51},
};

inline constexpr CommandDesc tcl_chan_cmd{
    .name = "chan",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_chan_subcmds,
    .options = tcl_chan_opts,
    .forms = tcl_chan_forms,
    .side_effect_hints = tcl_chan_effects,
    .hover_index = 52,
};

// clock
inline constexpr OptionDesc tcl_clock_opts[] = {
    {.name = "-base", .takes_value = true, .value_hint = "timeVal", .detail = "Base time for relative scanning."},
    {.name = "-format", .takes_value = true, .value_hint = "format", .detail = "strftime-style format string."},
    {.name = "-gmt", .takes_value = true, .value_hint = "boolean", .detail = "Use GMT instead of local time."},
    {.name = "-locale", .takes_value = true, .value_hint = "locale", .detail = "Locale for month/day names."},
    {.name = "-timezone", .takes_value = true, .value_hint = "zone", .detail = "Time zone for conversion."},
    {.name = "-validate", .takes_value = true, .value_hint = "boolean", .detail = "Validate date fields strictly."},
};

inline constexpr FormDesc tcl_clock_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_clock_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr SubCmdDesc tcl_clock_subcmds[] = {
    {.name = "add", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .pure = true, .return_type = TclType::INT, .hover_index = 53},
    {.name = "clicks", .arity = {0, 1}, .return_type = TclType::INT, .hover_index = 54},
    {.name = "format", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .pure = true, .hover_index = 55},
    {.name = "microseconds", .arity = {0, 0}, .return_type = TclType::INT, .hover_index = 56},
    {.name = "milliseconds", .arity = {0, 0}, .return_type = TclType::INT, .hover_index = 57},
    {.name = "scan", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .pure = true, .return_type = TclType::INT, .hover_index = 58},
    {.name = "seconds", .arity = {0, 0}, .return_type = TclType::INT, .hover_index = 59},
};

inline constexpr CommandDesc tcl_clock_cmd{
    .name = "clock",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_clock_subcmds,
    .options = tcl_clock_opts,
    .forms = tcl_clock_forms,
    .side_effect_hints = tcl_clock_effects,
    .hover_index = 60,
};

// close
inline constexpr ArgPattern tcl_close_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
};

inline constexpr FormDesc tcl_close_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_close_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .writes = true},
};

inline constexpr CommandDesc tcl_close_cmd{
    .name = "close",
    .arity = {1, 2},
    .arg_patterns = tcl_close_args,
    .forms = tcl_close_forms,
    .side_effect_hints = tcl_close_effects,
    .hover_index = 61,
};

// concat
inline constexpr FormDesc tcl_concat_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_concat_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_concat_cmd{
    .name = "concat",
    .forms = tcl_concat_forms,
    .pure = true,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_concat_effects,
    .hover_index = 62,
};

// continue
inline constexpr FormDesc tcl_continue_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_continue_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_continue_cmd{
    .name = "continue",
    .arity = {0, 0},
    .forms = tcl_continue_forms,
    .needs_start_cmd = true,
    .side_effect_hints = tcl_continue_effects,
    .hover_index = 63,
};

// coroutine
inline constexpr ArgPattern tcl_coroutine_args[] = {
    {.kind = FIXED, .role = NAME, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
};

inline constexpr FormDesc tcl_coroutine_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_coroutine_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_coroutine_cmd{
    .name = "coroutine",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_coroutine_args,
    .forms = tcl_coroutine_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_coroutine_effects,
    .hover_index = 64,
};

// dict
inline constexpr FormDesc tcl_dict_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_dict_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true},
};

inline constexpr ArgPattern tcl_dict_sub_append_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgPattern tcl_dict_sub_for_args[] = {
    {.kind = FIXED, .role = BODY, .index = 2},
};

inline constexpr ArgPattern tcl_dict_sub_incr_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgPattern tcl_dict_sub_lappend_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgPattern tcl_dict_sub_map_args[] = {
    {.kind = FIXED, .role = BODY, .index = 2},
};

inline constexpr ArgPattern tcl_dict_sub_set_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgPattern tcl_dict_sub_unset_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgPattern tcl_dict_sub_update_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgPattern tcl_dict_sub_with_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgTypeDesc tcl_dict_sub_get_types[] = {
    {.index = 0, .expected = TclType::DICT},
};

inline constexpr ArgTypeDesc tcl_dict_sub_keys_types[] = {
    {.index = 0, .expected = TclType::DICT},
};

inline constexpr ArgTypeDesc tcl_dict_sub_set_types[] = {
    {.index = 0, .expected = TclType::DICT},
};

inline constexpr ArgTypeDesc tcl_dict_sub_size_types[] = {
    {.index = 0, .expected = TclType::DICT},
};

inline constexpr ArgTypeDesc tcl_dict_sub_values_types[] = {
    {.index = 0, .expected = TclType::DICT},
};

inline constexpr SubCmdDesc tcl_dict_subcmds[] = {
    {.name = "append", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_dict_sub_append_args, .hover_index = 65},
    {.name = "create", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .pure = true, .return_type = TclType::DICT, .hover_index = 66},
    {.name = "exists", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .pure = true, .return_type = TclType::BOOLEAN, .hover_index = 67},
    {.name = "filter", .arity = {3, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::DICT, .hover_index = 68},
    {.name = "for", .arity = {3, 3}, .arg_patterns = tcl_dict_sub_for_args, .loop_list_header = true, .hover_index = 69},
    {.name = "get", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .arg_types = tcl_dict_sub_get_types, .pure = true, .hover_index = 70},
    {.name = "incr", .arity = {2, 3}, .arg_patterns = tcl_dict_sub_incr_args, .return_type = TclType::INT, .hover_index = 71},
    {.name = "info", .arity = {1, 1}, .hover_index = 72},
    {.name = "keys", .arity = {1, 2}, .arg_types = tcl_dict_sub_keys_types, .pure = true, .return_type = TclType::LIST, .hover_index = 73},
    {.name = "lappend", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_dict_sub_lappend_args, .hover_index = 74},
    {.name = "map", .arity = {3, 3}, .arg_patterns = tcl_dict_sub_map_args, .loop_list_header = true, .hover_index = 75},
    {.name = "merge", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .pure = true, .return_type = TclType::DICT, .hover_index = 76},
    {.name = "remove", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::DICT, .hover_index = 77},
    {.name = "replace", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::DICT, .hover_index = 78},
    {.name = "set", .arity = {3, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_dict_sub_set_args, .arg_types = tcl_dict_sub_set_types, .return_type = TclType::DICT, .hover_index = 79},
    {.name = "size", .arity = {1, 1}, .arg_types = tcl_dict_sub_size_types, .pure = true, .return_type = TclType::INT, .hover_index = 80},
    {.name = "unset", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_dict_sub_unset_args, .hover_index = 81},
    {.name = "update", .arity = {3, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_dict_sub_update_args, .hover_index = 82},
    {.name = "values", .arity = {1, 2}, .arg_types = tcl_dict_sub_values_types, .pure = true, .return_type = TclType::LIST, .hover_index = 83},
    {.name = "with", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_dict_sub_with_args, .hover_index = 84},
};

inline constexpr CommandDesc tcl_dict_cmd{
    .name = "dict",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_dict_subcmds,
    .forms = tcl_dict_forms,
    .dialects = DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90,
    .never_inline_body = true,
    .cse_candidate = true,
    .side_effect_hints = tcl_dict_effects,
    .hover_index = 85,
};

// encoding
inline constexpr OptionDesc tcl_encoding_opts[] = {
    {.name = "-profile", .takes_value = true, .value_hint = "profile", .detail = "Encoding profile (strict, tcl8, replace)."},
};

inline constexpr FormDesc tcl_encoding_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_encoding_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr SubCmdDesc tcl_encoding_subcmds[] = {
    {.name = "convertfrom", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 86},
    {.name = "convertto", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::BYTEARRAY, .hover_index = 87},
    {.name = "dirs", .arity = {0, 1}, .hover_index = 88},
    {.name = "names", .arity = {0, 0}, .return_type = TclType::LIST, .hover_index = 89},
    {.name = "profiles", .arity = {0, 0}, .hover_index = 90},
    {.name = "system", .arity = {0, 1}, .hover_index = 91},
    {.name = "user", .arity = {0, 1}, .hover_index = 92},
};

inline constexpr CommandDesc tcl_encoding_cmd{
    .name = "encoding",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_encoding_subcmds,
    .options = tcl_encoding_opts,
    .forms = tcl_encoding_forms,
    .side_effect_hints = tcl_encoding_effects,
    .hover_index = 93,
};

// eof
inline constexpr ArgPattern tcl_eof_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
};

inline constexpr FormDesc tcl_eof_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_eof_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true},
};

inline constexpr CommandDesc tcl_eof_cmd{
    .name = "eof",
    .arity = {1, 1},
    .arg_patterns = tcl_eof_args,
    .forms = tcl_eof_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .return_type = TclType::BOOLEAN,
    .side_effect_hints = tcl_eof_effects,
    .hover_index = 94,
};

// error
inline constexpr FormDesc tcl_error_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_error_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_error_cmd{
    .name = "error",
    .arity = {1, 3},
    .forms = tcl_error_forms,
    .needs_start_cmd = true,
    .side_effect_hints = tcl_error_effects,
    .hover_index = 95,
};

// eval
inline constexpr ArgPattern tcl_eval_args[] = {
    {.kind = TAIL, .role = BODY, .index = 0},
};

inline constexpr FormDesc tcl_eval_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_eval_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_eval_cmd{
    .name = "eval",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_eval_args,
    .forms = tcl_eval_forms,
    .creates_dynamic_barrier = true,
    .taint_sink = true,
    .xc_translatable = -1,
    .side_effect_hints = tcl_eval_effects,
    .taint_hints = {.safe_colour = TaintColour::TAINTED},
    .hover_index = 96,
};

// exec
inline constexpr OptionDesc tcl_exec_opts[] = {
    {.name = "-ignorestderr"},
    {.name = "-keepnewline"},
    {.name = "--", .is_terminator = true},
};

inline constexpr FormDesc tcl_exec_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_exec_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_exec_cmd{
    .name = "exec",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = tcl_exec_opts,
    .forms = tcl_exec_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .taint_sink = true,
    .side_effect_hints = tcl_exec_effects,
    .taint_hints = {.safe_colour = TaintColour::TAINTED},
};

// exit
inline constexpr FormDesc tcl_exit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_exit_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_exit_cmd{
    .name = "exit",
    .arity = {0, 1},
    .forms = tcl_exit_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_exit_effects,
    .hover_index = 97,
};

// expr
inline constexpr ArgPattern tcl_expr_args[] = {
    {.kind = FIXED, .role = EXPR, .index = 0},
};

inline constexpr ArgTypeDesc tcl_expr_types[] = {
    {.index = 0, .expected = TclType::NUMERIC, .shimmers = true},
};

inline constexpr FormDesc tcl_expr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_expr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_expr_cmd{
    .name = "expr",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_expr_args,
    .forms = tcl_expr_forms,
    .needs_start_cmd = true,
    .taint_sink = true,
    .arg_types = tcl_expr_types,
    .return_type = TclType::NUMERIC,
    .side_effect_hints = tcl_expr_effects,
    .hover_index = 98,
};

// fblocked
inline constexpr FormDesc tcl_fblocked_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_fblocked_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true},
};

inline constexpr CommandDesc tcl_fblocked_cmd{
    .name = "fblocked",
    .arity = {1, 1},
    .forms = tcl_fblocked_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .return_type = TclType::BOOLEAN,
    .side_effect_hints = tcl_fblocked_effects,
    .hover_index = 99,
};

// fconfigure
inline constexpr ArgPattern tcl_fconfigure_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
};

inline constexpr OptionDesc tcl_fconfigure_opts[] = {
    {.name = "-blocking", .takes_value = true, .value_hint = "boolean"},
    {.name = "-buffering", .takes_value = true, .value_hint = "mode"},
    {.name = "-buffersize", .takes_value = true, .value_hint = "size"},
    {.name = "-encoding", .takes_value = true, .value_hint = "encoding"},
    {.name = "-eofchar", .takes_value = true, .value_hint = "chars"},
    {.name = "-translation", .takes_value = true, .value_hint = "mode"},
};

inline constexpr FormDesc tcl_fconfigure_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_fconfigure_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_fconfigure_cmd{
    .name = "fconfigure",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_fconfigure_args,
    .options = tcl_fconfigure_opts,
    .forms = tcl_fconfigure_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_fconfigure_effects,
    .hover_index = 100,
};

// fcopy
inline constexpr FormDesc tcl_fcopy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_fcopy_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_fcopy_cmd{
    .name = "fcopy",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_fcopy_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_fcopy_effects,
    .hover_index = 101,
};

// file
inline constexpr FormDesc tcl_file_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_file_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .writes = true},
};

inline constexpr ArgPattern tcl_file_sub_lstat_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 1},
};

inline constexpr ArgPattern tcl_file_sub_stat_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 1},
};

inline constexpr ArgPattern tcl_file_sub_tempfile_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr SubCmdDesc tcl_file_subcmds[] = {
    {.name = "atime", .arity = {1, 2}, .return_type = TclType::INT, .hover_index = 102},
    {.name = "attributes", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::LIST, .hover_index = 103},
    {.name = "channels", .arity = {0, 1}, .return_type = TclType::LIST, .hover_index = 104},
    {.name = "copy", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 105},
    {.name = "delete", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .destructive = true, .hover_index = 106},
    {.name = "dirname", .arity = {1, 1}, .hover_index = 107},
    {.name = "executable", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 108},
    {.name = "exists", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 109},
    {.name = "extension", .arity = {1, 1}, .hover_index = 110},
    {.name = "isdirectory", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 111},
    {.name = "isfile", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 112},
    {.name = "join", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .taint_hints = {.transform_colour = TaintColour::TAINTED}, .hover_index = 113},
    {.name = "link", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 114},
    {.name = "lstat", .arity = {2, 2}, .arg_patterns = tcl_file_sub_lstat_args, .hover_index = 115},
    {.name = "mkdir", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .destructive = true, .hover_index = 116},
    {.name = "mtime", .arity = {1, 2}, .return_type = TclType::INT, .hover_index = 117},
    {.name = "nativename", .arity = {1, 1}, .hover_index = 118},
    {.name = "normalize", .arity = {1, 1}, .taint_hints = {.transform_colour = TaintColour::TAINTED}, .hover_index = 119},
    {.name = "owned", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 120},
    {.name = "pathtype", .arity = {1, 1}, .hover_index = 121},
    {.name = "readable", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 122},
    {.name = "readlink", .arity = {1, 1}, .hover_index = 123},
    {.name = "rename", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .destructive = true, .hover_index = 124},
    {.name = "rootname", .arity = {1, 1}, .hover_index = 125},
    {.name = "separator", .arity = {0, 1}, .hover_index = 126},
    {.name = "size", .arity = {1, 1}, .return_type = TclType::INT, .hover_index = 127},
    {.name = "split", .arity = {1, 1}, .return_type = TclType::LIST, .hover_index = 128},
    {.name = "stat", .arity = {2, 2}, .arg_patterns = tcl_file_sub_stat_args, .hover_index = 129},
    {.name = "system", .arity = {1, 1}, .hover_index = 130},
    {.name = "tail", .arity = {1, 1}, .hover_index = 131},
    {.name = "tempfile", .arity = {0, 2}, .arg_patterns = tcl_file_sub_tempfile_args, .hover_index = 132},
    {.name = "type", .arity = {1, 1}, .hover_index = 133},
    {.name = "volumes", .arity = {0, 0}, .return_type = TclType::LIST, .hover_index = 134},
    {.name = "writable", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 135},
};

inline constexpr CommandDesc tcl_file_cmd{
    .name = "file",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_file_subcmds,
    .forms = tcl_file_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_file_effects,
    .hover_index = 136,
};

// fileevent
inline constexpr ArgPattern tcl_fileevent_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
    {.kind = FIXED, .role = BODY, .index = 2},
};

inline constexpr FormDesc tcl_fileevent_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_fileevent_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .writes = true},
};

inline constexpr CommandDesc tcl_fileevent_cmd{
    .name = "fileevent",
    .arity = {2, 3},
    .arg_patterns = tcl_fileevent_args,
    .forms = tcl_fileevent_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_fileevent_effects,
    .hover_index = 137,
};

// filename
inline constexpr CommandDesc tcl_filename_cmd{
    .name = "filename",
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 138,
};

// flush
inline constexpr ArgPattern tcl_flush_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
};

inline constexpr FormDesc tcl_flush_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_flush_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .writes = true},
};

inline constexpr CommandDesc tcl_flush_cmd{
    .name = "flush",
    .arity = {1, 1},
    .arg_patterns = tcl_flush_args,
    .forms = tcl_flush_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_flush_effects,
    .hover_index = 139,
};

// for
inline constexpr ArgPattern tcl_for_args[] = {
    {.kind = FIXED, .role = BODY, .index = 0},
    {.kind = FIXED, .role = EXPR, .index = 1},
    {.kind = FIXED, .role = BODY, .index = 2},
    {.kind = FIXED, .role = BODY, .index = 3},
};

inline constexpr ArgTypeDesc tcl_for_types[] = {
    {.index = 1, .expected = TclType::BOOLEAN, .shimmers = true},
};

inline constexpr FormDesc tcl_for_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_for_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_for_cmd{
    .name = "for",
    .arity = {4, 4},
    .arg_patterns = tcl_for_args,
    .forms = tcl_for_forms,
    .is_control_flow = true,
    .never_inline_body = true,
    .has_loop_body = true,
    .arg_types = tcl_for_types,
    .side_effect_hints = tcl_for_effects,
    .hover_index = 140,
};

// foreach
inline constexpr ArgPattern tcl_foreach_args[] = {
    {.kind = STRIDE, .role = VALUE, .index = 0, .stride = 2},
    {.kind = STRIDE, .role = VALUE, .index = 1, .stride = 2},
    {.kind = FIXED, .role = BODY, .index = -1},
};

inline constexpr FormDesc tcl_foreach_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_foreach_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_foreach_cmd{
    .name = "foreach",
    .arity = {3, std::numeric_limits<std::int32_t>::max(), 2, 1},
    .arg_patterns = tcl_foreach_args,
    .layout_resolver = LayoutResolver::FOREACH,
    .forms = tcl_foreach_forms,
    .is_control_flow = true,
    .never_inline_body = true,
    .has_loop_body = true,
    .loop_list_header = true,
    .side_effect_hints = tcl_foreach_effects,
    .hover_index = 141,
};

// format
inline constexpr ArgPattern tcl_format_args[] = {
    {.kind = FIXED, .role = VALUE, .index = 0},
    {.kind = TAIL, .role = VALUE, .index = 1},
};

inline constexpr FormDesc tcl_format_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_format_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_format_cmd{
    .name = "format",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_format_args,
    .forms = tcl_format_forms,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = tcl_format_effects,
    .hover_index = 142,
};

// gets
inline constexpr ArgPattern tcl_gets_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
    {.kind = FIXED, .role = VAR_NAME, .index = 1},
};

inline constexpr FormDesc tcl_gets_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_gets_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_gets_cmd{
    .name = "gets",
    .arity = {1, 2},
    .arg_patterns = tcl_gets_args,
    .forms = tcl_gets_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_gets_effects,
    .hover_index = 143,
};

// glob
inline constexpr OptionDesc tcl_glob_opts[] = {
    {.name = "-directory", .takes_value = true, .value_hint = "dir"},
    {.name = "-join"},
    {.name = "-nocomplain"},
    {.name = "-path", .takes_value = true, .value_hint = "pathPrefix"},
    {.name = "-tails"},
    {.name = "-types", .takes_value = true, .value_hint = "typeList"},
    {.name = "--", .is_terminator = true},
};

inline constexpr FormDesc tcl_glob_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_glob_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true},
};

inline constexpr CommandDesc tcl_glob_cmd{
    .name = "glob",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = tcl_glob_opts,
    .forms = tcl_glob_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_glob_effects,
    .hover_index = 144,
};

// global
inline constexpr ArgPattern tcl_global_args[] = {
    {.kind = TAIL, .role = VAR_NAME, .index = 0},
};

inline constexpr FormDesc tcl_global_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_global_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .writes = true},
};

inline constexpr CommandDesc tcl_global_cmd{
    .name = "global",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_global_args,
    .forms = tcl_global_forms,
    .side_effect_hints = tcl_global_effects,
    .hover_index = 145,
};

// http
inline constexpr CommandDesc tcl_http_cmd{
    .name = "http",
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 146,
};

// if
inline constexpr ArgPattern tcl_if_args[] = {
    {.kind = FIXED, .role = EXPR, .index = 0},
    {.kind = FIXED, .role = BODY, .index = 1},
    {.kind = FIXED, .role = BODY, .index = 3},
    {.kind = FIXED, .role = BODY, .index = 5},
};

inline constexpr ArgTypeDesc tcl_if_types[] = {
    {.index = 0, .expected = TclType::BOOLEAN, .shimmers = true},
};

inline constexpr FormDesc tcl_if_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_if_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr std::string_view tcl_if_keywords[] = {
    "then",
    "else",
    "elseif",
};

inline constexpr CommandDesc tcl_if_cmd{
    .name = "if",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_if_args,
    .layout_resolver = LayoutResolver::IF,
    .forms = tcl_if_forms,
    .is_control_flow = true,
    .never_inline_body = true,
    .arg_types = tcl_if_types,
    .side_effect_hints = tcl_if_effects,
    .hover_index = 147,
    .formatting_keywords = tcl_if_keywords,
};

// incr
inline constexpr ArgPattern tcl_incr_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
};

inline constexpr ArgTypeDesc tcl_incr_types[] = {
    {.index = 0, .expected = TclType::INT, .shimmers = true},
    {.index = 1, .expected = TclType::INT},
};

inline constexpr FormDesc tcl_incr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_incr_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_incr_cmd{
    .name = "incr",
    .arity = {1, 2},
    .arg_patterns = tcl_incr_args,
    .forms = tcl_incr_forms,
    .assigns_variable_at = 0,
    .arg_types = tcl_incr_types,
    .return_type = TclType::INT,
    .side_effect_hints = tcl_incr_effects,
    .hover_index = 148,
};

// info
inline constexpr FormDesc tcl_info_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_info_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true},
};

inline constexpr ArgPattern tcl_info_sub_default_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 2},
};

inline constexpr ArgPattern tcl_info_sub_exists_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr SubCmdDesc tcl_info_subcmds[] = {
    {.name = "args", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::LIST},
    {.name = "body", .arity = {1, std::numeric_limits<std::int32_t>::max()}},
    {.name = "cmdcount", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::INT},
    {.name = "commands", .arity = {0, 1}, .return_type = TclType::LIST},
    {.name = "complete", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::BOOLEAN},
    {.name = "default", .arity = {3, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_info_sub_default_args, .return_type = TclType::BOOLEAN},
    {.name = "exists", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_info_sub_exists_args, .return_type = TclType::BOOLEAN},
    {.name = "frame", .arity = {0, 1}, .return_type = TclType::DICT},
    {.name = "functions", .arity = {0, 1}, .return_type = TclType::LIST},
    {.name = "globals", .arity = {0, 1}, .return_type = TclType::LIST},
    {.name = "hostname", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.name = "level", .arity = {0, 1}, .return_type = TclType::INT},
    {.name = "library", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.name = "loaded", .arity = {0, 1}, .return_type = TclType::LIST},
    {.name = "locals", .arity = {0, 1}, .return_type = TclType::LIST},
    {.name = "nameofexecutable", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.name = "patchlevel", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.name = "procs", .arity = {0, 1}, .return_type = TclType::LIST},
    {.name = "script", .arity = {0, 1}},
    {.name = "sharedlibextension", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.name = "tclversion", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.name = "vars", .arity = {0, 1}, .return_type = TclType::LIST},
};

inline constexpr CommandDesc tcl_info_cmd{
    .name = "info",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_info_subcmds,
    .forms = tcl_info_forms,
    .side_effect_hints = tcl_info_effects,
    .hover_index = 149,
};

// interp
inline constexpr FormDesc tcl_interp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_interp_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr ArgPattern tcl_interp_sub_eval_args[] = {
    {.kind = FIXED, .role = BODY, .index = 1},
};

inline constexpr SubCmdDesc tcl_interp_subcmds[] = {
    {.name = "alias", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 150},
    {.name = "aliases", .arity = {0, 1}, .return_type = TclType::LIST, .hover_index = 151},
    {.name = "create", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .hover_index = 152},
    {.name = "delete", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 153},
    {.name = "eval", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_interp_sub_eval_args, .hover_index = 154},
    {.name = "exists", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 155},
    {.name = "expose", .arity = {2, 3}, .hover_index = 156},
    {.name = "hidden", .arity = {0, 1}, .return_type = TclType::LIST, .hover_index = 157},
    {.name = "hide", .arity = {2, 3}, .hover_index = 158},
    {.name = "invokehidden", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 159},
    {.name = "issafe", .arity = {0, 1}, .return_type = TclType::BOOLEAN, .hover_index = 160},
    {.name = "limit", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 161},
    {.name = "marktrusted", .arity = {1, 1}, .hover_index = 162},
    {.name = "recursionlimit", .arity = {1, 2}, .return_type = TclType::INT, .hover_index = 163},
    {.name = "share", .arity = {3, 3}, .hover_index = 164},
    {.name = "slaves", .arity = {0, 1}, .return_type = TclType::LIST, .hover_index = 165},
    {.name = "target", .arity = {2, 2}, .hover_index = 166},
    {.name = "transfer", .arity = {3, 3}, .hover_index = 167},
};

inline constexpr CommandDesc tcl_interp_cmd{
    .name = "interp",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_interp_subcmds,
    .forms = tcl_interp_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_interp_effects,
    .hover_index = 168,
};

// join
inline constexpr ArgTypeDesc tcl_join_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_join_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_join_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_join_cmd{
    .name = "join",
    .arity = {1, 2},
    .forms = tcl_join_forms,
    .pure = true,
    .cse_candidate = true,
    .arg_types = tcl_join_types,
    .side_effect_hints = tcl_join_effects,
    .hover_index = 169,
};

// lappend
inline constexpr ArgPattern tcl_lappend_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgTypeDesc tcl_lappend_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_lappend_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lappend_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_lappend_cmd{
    .name = "lappend",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_lappend_args,
    .forms = tcl_lappend_forms,
    .assigns_variable_at = 0,
    .arg_types = tcl_lappend_types,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lappend_effects,
    .hover_index = 170,
};

// lassign
inline constexpr ArgPattern tcl_lassign_args[] = {
    {.kind = FIXED, .role = VALUE, .index = 0},
    {.kind = TAIL, .role = VAR_NAME, .index = 1},
};

inline constexpr ArgTypeDesc tcl_lassign_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_lassign_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lassign_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .writes = true},
};

inline constexpr CommandDesc tcl_lassign_cmd{
    .name = "lassign",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_lassign_args,
    .forms = tcl_lassign_forms,
    .dialects = DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90,
    .arg_types = tcl_lassign_types,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lassign_effects,
    .hover_index = 171,
};

// lindex
inline constexpr ArgTypeDesc tcl_lindex_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_lindex_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lindex_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_lindex_cmd{
    .name = "lindex",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_lindex_forms,
    .pure = true,
    .cse_candidate = true,
    .arg_types = tcl_lindex_types,
    .side_effect_hints = tcl_lindex_effects,
    .hover_index = 172,
};

// linsert
inline constexpr ArgTypeDesc tcl_linsert_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_linsert_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_linsert_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_linsert_cmd{
    .name = "linsert",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_linsert_forms,
    .arg_types = tcl_linsert_types,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_linsert_effects,
    .hover_index = 173,
};

// list
inline constexpr FormDesc tcl_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_list_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_list_cmd{
    .name = "list",
    .forms = tcl_list_forms,
    .pure = true,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_list_effects,
    .hover_index = 174,
};

// llength
inline constexpr ArgTypeDesc tcl_llength_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_llength_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_llength_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_llength_cmd{
    .name = "llength",
    .arity = {1, 1},
    .forms = tcl_llength_forms,
    .pure = true,
    .cse_candidate = true,
    .arg_types = tcl_llength_types,
    .return_type = TclType::INT,
    .side_effect_hints = tcl_llength_effects,
    .hover_index = 175,
};

// lmap
inline constexpr ArgPattern tcl_lmap_args[] = {
    {.kind = STRIDE, .role = VALUE, .index = 0, .stride = 2},
    {.kind = STRIDE, .role = VALUE, .index = 1, .stride = 2},
    {.kind = FIXED, .role = BODY, .index = -1},
};

inline constexpr FormDesc tcl_lmap_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lmap_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_lmap_cmd{
    .name = "lmap",
    .arity = {3, std::numeric_limits<std::int32_t>::max(), 2, 1},
    .arg_patterns = tcl_lmap_args,
    .layout_resolver = LayoutResolver::FOREACH,
    .forms = tcl_lmap_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .loop_list_header = true,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lmap_effects,
    .hover_index = 176,
};

// load
inline constexpr FormDesc tcl_load_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_load_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_load_cmd{
    .name = "load",
    .arity = {1, 3},
    .forms = tcl_load_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_load_effects,
    .hover_index = 177,
};

// lrange
inline constexpr ArgTypeDesc tcl_lrange_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_lrange_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lrange_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_lrange_cmd{
    .name = "lrange",
    .arity = {3, 3},
    .forms = tcl_lrange_forms,
    .pure = true,
    .cse_candidate = true,
    .arg_types = tcl_lrange_types,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lrange_effects,
    .hover_index = 178,
};

// lrepeat
inline constexpr ArgTypeDesc tcl_lrepeat_types[] = {
    {.index = 0, .expected = TclType::INT},
};

inline constexpr FormDesc tcl_lrepeat_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lrepeat_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_lrepeat_cmd{
    .name = "lrepeat",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_lrepeat_forms,
    .pure = true,
    .arg_types = tcl_lrepeat_types,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lrepeat_effects,
    .hover_index = 179,
};

// lreplace
inline constexpr ArgTypeDesc tcl_lreplace_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_lreplace_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lreplace_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_lreplace_cmd{
    .name = "lreplace",
    .arity = {3, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_lreplace_forms,
    .arg_types = tcl_lreplace_types,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lreplace_effects,
    .hover_index = 180,
};

// lreverse
inline constexpr ArgTypeDesc tcl_lreverse_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_lreverse_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lreverse_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_lreverse_cmd{
    .name = "lreverse",
    .arity = {1, 1},
    .forms = tcl_lreverse_forms,
    .pure = true,
    .arg_types = tcl_lreverse_types,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lreverse_effects,
    .hover_index = 181,
};

// lsearch
inline constexpr ArgPattern tcl_lsearch_args[] = {
    {.kind = FIXED, .role = VALUE, .index = 0},
    {.kind = FIXED, .role = PATTERN, .index = 1},
};

inline constexpr OptionDesc tcl_lsearch_opts[] = {
    {.name = "-exact"},
    {.name = "-glob"},
    {.name = "-regexp"},
    {.name = "-sorted"},
    {.name = "-all"},
    {.name = "-inline"},
    {.name = "-not"},
    {.name = "-start", .takes_value = true, .value_hint = "index"},
    {.name = "-ascii"},
    {.name = "-dictionary"},
    {.name = "-integer"},
    {.name = "-real"},
    {.name = "-nocase"},
    {.name = "-increasing"},
    {.name = "-decreasing"},
    {.name = "-bisect"},
    {.name = "-index", .takes_value = true, .value_hint = "index"},
    {.name = "-subindices"},
    {.name = "--", .is_terminator = true, .detail = "End of options."},
};

inline constexpr FormDesc tcl_lsearch_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lsearch_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_lsearch_cmd{
    .name = "lsearch",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_lsearch_args,
    .options = tcl_lsearch_opts,
    .forms = tcl_lsearch_forms,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = tcl_lsearch_effects,
};

// lset
inline constexpr ArgPattern tcl_lset_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr ArgTypeDesc tcl_lset_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr FormDesc tcl_lset_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lset_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .writes = true},
};

inline constexpr CommandDesc tcl_lset_cmd{
    .name = "lset",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_lset_args,
    .forms = tcl_lset_forms,
    .arg_types = tcl_lset_types,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lset_effects,
    .hover_index = 182,
};

// lsort
inline constexpr OptionDesc tcl_lsort_opts[] = {
    {.name = "-ascii"},
    {.name = "-dictionary"},
    {.name = "-integer"},
    {.name = "-real"},
    {.name = "-nocase"},
    {.name = "-increasing"},
    {.name = "-decreasing"},
    {.name = "-indices"},
    {.name = "-unique"},
    {.name = "-command", .takes_value = true, .value_hint = "cmdPrefix"},
    {.name = "-index", .takes_value = true, .value_hint = "index"},
    {.name = "-stride", .takes_value = true, .value_hint = "length"},
};

inline constexpr FormDesc tcl_lsort_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_lsort_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_lsort_cmd{
    .name = "lsort",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = tcl_lsort_opts,
    .forms = tcl_lsort_forms,
    .pure = true,
    .cse_candidate = true,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_lsort_effects,
};

// memory
inline constexpr CommandDesc tcl_memory_cmd{
    .name = "memory",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 183,
};

// namespace
inline constexpr FormDesc tcl_namespace_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_namespace_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true},
};

inline constexpr ArgPattern tcl_namespace_sub_code_args[] = {
    {.kind = FIXED, .role = BODY, .index = 0},
};

inline constexpr ArgPattern tcl_namespace_sub_eval_args[] = {
    {.kind = FIXED, .role = NAME, .index = 0},
    {.kind = FIXED, .role = BODY, .index = 1},
};

inline constexpr ArgPattern tcl_namespace_sub_inscope_args[] = {
    {.kind = FIXED, .role = NAME, .index = 0},
    {.kind = FIXED, .role = BODY, .index = 1},
};

inline constexpr SubCmdDesc tcl_namespace_subcmds[] = {
    {.name = "children", .arity = {0, 2}, .return_type = TclType::LIST, .hover_index = 184},
    {.name = "code", .arity = {1, 1}, .arg_patterns = tcl_namespace_sub_code_args, .hover_index = 185},
    {.name = "current", .arity = {0, 0}, .hover_index = 186},
    {.name = "delete", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .hover_index = 187},
    {.name = "ensemble", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 188},
    {.name = "eval", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_namespace_sub_eval_args, .hover_index = 189},
    {.name = "exists", .arity = {1, 1}, .return_type = TclType::BOOLEAN, .hover_index = 190},
    {.name = "export", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .hover_index = 191},
    {.name = "import", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .hover_index = 192},
    {.name = "inscope", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .arg_patterns = tcl_namespace_sub_inscope_args, .hover_index = 193},
    {.name = "origin", .arity = {1, 1}, .hover_index = 194},
    {.name = "parent", .arity = {0, 1}, .hover_index = 195},
    {.name = "path", .arity = {0, 1}, .return_type = TclType::LIST, .hover_index = 196},
    {.name = "qualifiers", .arity = {1, 1}, .hover_index = 197},
    {.name = "tail", .arity = {1, 1}, .hover_index = 198},
    {.name = "unknown", .arity = {0, 1}, .hover_index = 199},
    {.name = "upvar", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .creates_scope_alias = true, .hover_index = 200},
    {.name = "which", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 201},
};

inline constexpr CommandDesc tcl_namespace_cmd{
    .name = "namespace",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_namespace_subcmds,
    .forms = tcl_namespace_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .never_inline_body = true,
    .side_effect_hints = tcl_namespace_effects,
    .hover_index = 202,
};

// oo::abstract
inline constexpr FormDesc tcl_oo__abstract_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_oo__abstract_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_oo__abstract_cmd{
    .name = "oo::abstract",
    .forms = tcl_oo__abstract_forms,
    .dialects = DialectFlags::TCL90,
    .side_effect_hints = tcl_oo__abstract_effects,
    .hover_index = 203,
};

// oo::class
inline constexpr FormDesc tcl_oo__class_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_oo__class_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_oo__class_cmd{
    .name = "oo::class",
    .layout_resolver = LayoutResolver::OO_CLASS,
    .forms = tcl_oo__class_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_oo__class_effects,
    .hover_index = 204,
};

// oo::configurable
inline constexpr FormDesc tcl_oo__configurable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_oo__configurable_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_oo__configurable_cmd{
    .name = "oo::configurable",
    .forms = tcl_oo__configurable_forms,
    .dialects = DialectFlags::TCL90,
    .side_effect_hints = tcl_oo__configurable_effects,
    .hover_index = 205,
};

// oo::copy
inline constexpr FormDesc tcl_oo__copy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_oo__copy_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_oo__copy_cmd{
    .name = "oo::copy",
    .arity = {2, 2},
    .forms = tcl_oo__copy_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_oo__copy_effects,
    .hover_index = 206,
};

// oo::define
inline constexpr FormDesc tcl_oo__define_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_oo__define_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_oo__define_cmd{
    .name = "oo::define",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .layout_resolver = LayoutResolver::OO_DEFINE,
    .forms = tcl_oo__define_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .is_scope_barrier = true,
    .side_effect_hints = tcl_oo__define_effects,
    .hover_index = 207,
};

// oo::objdefine
inline constexpr FormDesc tcl_oo__objdefine_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_oo__objdefine_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_oo__objdefine_cmd{
    .name = "oo::objdefine",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_oo__objdefine_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_oo__objdefine_effects,
    .hover_index = 208,
};

// oo::object
inline constexpr FormDesc tcl_oo__object_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_oo__object_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true},
};

inline constexpr CommandDesc tcl_oo__object_cmd{
    .name = "oo::object",
    .forms = tcl_oo__object_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_oo__object_effects,
    .hover_index = 209,
};

// oo::singleton
inline constexpr FormDesc tcl_oo__singleton_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_oo__singleton_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_oo__singleton_cmd{
    .name = "oo::singleton",
    .forms = tcl_oo__singleton_forms,
    .dialects = DialectFlags::TCL90,
    .side_effect_hints = tcl_oo__singleton_effects,
    .hover_index = 210,
};

// open
inline constexpr FormDesc tcl_open_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_open_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .writes = true},
};

inline constexpr CommandDesc tcl_open_cmd{
    .name = "open",
    .arity = {1, 3},
    .forms = tcl_open_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_open_effects,
    .hover_index = 211,
};

// package
inline constexpr FormDesc tcl_package_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_package_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr ArgPattern tcl_package_sub_ifneeded_args[] = {
    {.kind = FIXED, .role = BODY, .index = 2},
};

inline constexpr SubCmdDesc tcl_package_subcmds[] = {
    {.name = "forget", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 212},
    {.name = "ifneeded", .arity = {2, 3}, .arg_patterns = tcl_package_sub_ifneeded_args, .hover_index = 213},
    {.name = "names", .arity = {0, 0}, .return_type = TclType::LIST, .hover_index = 214},
    {.name = "present", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 215},
    {.name = "provide", .arity = {1, 2}, .hover_index = 216},
    {.name = "require", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 217},
    {.name = "unknown", .arity = {0, 1}, .hover_index = 218},
    {.name = "vcompare", .arity = {2, 2}, .return_type = TclType::INT, .hover_index = 219},
    {.name = "versions", .arity = {1, 1}, .return_type = TclType::LIST, .hover_index = 220},
    {.name = "vsatisfies", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .return_type = TclType::BOOLEAN, .hover_index = 221},
};

inline constexpr CommandDesc tcl_package_cmd{
    .name = "package",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_package_subcmds,
    .forms = tcl_package_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_package_effects,
    .hover_index = 222,
};

// parray
inline constexpr ArgPattern tcl_parray_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr FormDesc tcl_parray_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_parray_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true},
};

inline constexpr CommandDesc tcl_parray_cmd{
    .name = "parray",
    .arity = {1, 1},
    .arg_patterns = tcl_parray_args,
    .forms = tcl_parray_forms,
    .side_effect_hints = tcl_parray_effects,
    .hover_index = 223,
};

// pid
inline constexpr FormDesc tcl_pid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_pid_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true},
};

inline constexpr CommandDesc tcl_pid_cmd{
    .name = "pid",
    .arity = {0, 1},
    .forms = tcl_pid_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_pid_effects,
    .hover_index = 224,
};

// pkg::create
inline constexpr CommandDesc tcl_pkg__create_cmd{
    .name = "pkg::create",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 225,
};

// pkg_mkindex
inline constexpr CommandDesc tcl_pkg_mkindex_cmd{
    .name = "pkg_mkindex",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 226,
};

// proc
inline constexpr ArgPattern tcl_proc_args[] = {
    {.kind = FIXED, .role = NAME, .index = 0},
    {.kind = FIXED, .role = PARAM_LIST, .index = 1},
    {.kind = FIXED, .role = BODY, .index = 2},
};

inline constexpr FormDesc tcl_proc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_proc_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true},
};

inline constexpr CommandDesc tcl_proc_cmd{
    .name = "proc",
    .arity = {3, 3},
    .arg_patterns = tcl_proc_args,
    .forms = tcl_proc_forms,
    .never_inline_body = true,
    .defines_procedure = true,
    .is_scope_barrier = true,
    .side_effect_hints = tcl_proc_effects,
    .hover_index = 227,
};

// puts
inline constexpr ArgPattern tcl_puts_args[] = {
    {.kind = FIXED, .role = VALUE, .index = 0},
};

inline constexpr OptionDesc tcl_puts_opts[] = {
    {.name = "-nonewline", .detail = "Do not append trailing newline."},
};

inline constexpr FormDesc tcl_puts_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_puts_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .writes = true},
};

inline constexpr CommandDesc tcl_puts_cmd{
    .name = "puts",
    .arity = {1, 2},
    .arg_patterns = tcl_puts_args,
    .options = tcl_puts_opts,
    .forms = tcl_puts_forms,
    .side_effect_hints = tcl_puts_effects,
    .taint_hints = {.output_sink_code = "T101"},
    .hover_index = 228,
};

// pwd
inline constexpr CommandDesc tcl_pwd_cmd{
    .name = "pwd",
    .arity = {0, 0},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 229,
};

// re_quote
inline constexpr FormDesc tcl_re_quote_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcl_re_quote_cmd{
    .name = "re_quote",
    .arity = {1, 1},
    .forms = tcl_re_quote_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 230,
};

// read
inline constexpr ArgPattern tcl_read_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
};

inline constexpr FormDesc tcl_read_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_read_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_read_cmd{
    .name = "read",
    .arity = {1, 2},
    .arg_patterns = tcl_read_args,
    .forms = tcl_read_forms,
    .side_effect_hints = tcl_read_effects,
    .hover_index = 231,
};

// regex::quote
inline constexpr FormDesc tcl_regex__quote_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcl_regex__quote_cmd{
    .name = "regex::quote",
    .arity = {1, 1},
    .forms = tcl_regex__quote_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 232,
};

// regex_quote
inline constexpr FormDesc tcl_regex_quote_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcl_regex_quote_cmd{
    .name = "regex_quote",
    .arity = {1, 1},
    .forms = tcl_regex_quote_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 233,
};

// regexp
inline constexpr ArgPattern tcl_regexp_args[] = {
    {.kind = FIXED, .role = PATTERN, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
    {.kind = TAIL, .role = VAR_NAME, .index = 2},
};

inline constexpr OptionDesc tcl_regexp_opts[] = {
    {.name = "-nocase"},
    {.name = "-expanded"},
    {.name = "-line"},
    {.name = "-linestop"},
    {.name = "-lineanchor"},
    {.name = "-all"},
    {.name = "-inline"},
    {.name = "-indices"},
    {.name = "-start", .takes_value = true, .value_hint = "index"},
    {.name = "--", .is_terminator = true},
};

inline constexpr FormDesc tcl_regexp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_regexp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_regexp_cmd{
    .name = "regexp",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_regexp_args,
    .options = tcl_regexp_opts,
    .forms = tcl_regexp_forms,
    .warn_without_terminator = true,
    .return_type = TclType::INT,
    .side_effect_hints = tcl_regexp_effects,
    .hover_index = 234,
    .pattern_type = PatternType::REGEX,
};

// regexp::quote
inline constexpr FormDesc tcl_regexp__quote_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcl_regexp__quote_cmd{
    .name = "regexp::quote",
    .arity = {1, 1},
    .forms = tcl_regexp__quote_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 235,
};

// regsub
inline constexpr ArgPattern tcl_regsub_args[] = {
    {.kind = FIXED, .role = PATTERN, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
    {.kind = FIXED, .role = VALUE, .index = 2},
    {.kind = FIXED, .role = VAR_NAME, .index = 3},
};

inline constexpr OptionDesc tcl_regsub_opts[] = {
    {.name = "-nocase"},
    {.name = "-expanded"},
    {.name = "-line"},
    {.name = "-linestop"},
    {.name = "-lineanchor"},
    {.name = "-all"},
    {.name = "-start", .takes_value = true, .value_hint = "index"},
    {.name = "--", .is_terminator = true},
};

inline constexpr FormDesc tcl_regsub_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_regsub_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_regsub_cmd{
    .name = "regsub",
    .arity = {3, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_regsub_args,
    .options = tcl_regsub_opts,
    .forms = tcl_regsub_forms,
    .return_type = TclType::INT,
    .side_effect_hints = tcl_regsub_effects,
    .hover_index = 236,
    .pattern_type = PatternType::REGEX,
};

// rename
inline constexpr ArgPattern tcl_rename_args[] = {
    {.kind = FIXED, .role = NAME, .index = 0},
    {.kind = FIXED, .role = NAME, .index = 1},
};

inline constexpr FormDesc tcl_rename_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_rename_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true},
};

inline constexpr CommandDesc tcl_rename_cmd{
    .name = "rename",
    .arity = {2, 2},
    .arg_patterns = tcl_rename_args,
    .forms = tcl_rename_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_rename_effects,
    .hover_index = 237,
};

// return
inline constexpr FormDesc tcl_return_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_return_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_return_cmd{
    .name = "return",
    .forms = tcl_return_forms,
    .needs_start_cmd = true,
    .side_effect_hints = tcl_return_effects,
    .hover_index = 238,
};

// scan
inline constexpr ArgPattern tcl_scan_args[] = {
    {.kind = FIXED, .role = VALUE, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
    {.kind = TAIL, .role = VAR_NAME, .index = 2},
};

inline constexpr FormDesc tcl_scan_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_scan_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_scan_cmd{
    .name = "scan",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_scan_args,
    .forms = tcl_scan_forms,
    .return_type = TclType::INT,
    .side_effect_hints = tcl_scan_effects,
    .hover_index = 239,
};

// seek
inline constexpr ArgPattern tcl_seek_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
};

inline constexpr FormDesc tcl_seek_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_seek_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .writes = true},
};

inline constexpr CommandDesc tcl_seek_cmd{
    .name = "seek",
    .arity = {2, 3},
    .arg_patterns = tcl_seek_args,
    .forms = tcl_seek_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_seek_effects,
    .hover_index = 240,
};

// set
inline constexpr ArgPattern tcl_set_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
};

inline constexpr FormDesc tcl_set_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_set_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_set_cmd{
    .name = "set",
    .arity = {1, 2},
    .arg_patterns = tcl_set_args,
    .forms = tcl_set_forms,
    .assigns_variable_at = 0,
    .side_effect_hints = tcl_set_effects,
    .hover_index = 241,
};

// socket
inline constexpr OptionDesc tcl_socket_opts[] = {
    {.name = "-myaddr", .takes_value = true, .value_hint = "addr", .detail = "Client-side local interface."},
    {.name = "-myport", .takes_value = true, .value_hint = "port", .detail = "Client-side local port."},
    {.name = "-async", .detail = "Connect asynchronously."},
    {.name = "-server", .takes_value = true, .value_hint = "command", .detail = "Server accept callback."},
};

inline constexpr FormDesc tcl_socket_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_socket_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true},
};

inline constexpr CommandDesc tcl_socket_cmd{
    .name = "socket",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .options = tcl_socket_opts,
    .forms = tcl_socket_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_socket_effects,
    .hover_index = 242,
};

// source
inline constexpr ArgPattern tcl_source_args[] = {
    {.kind = FIXED, .role = VALUE, .index = 0},
};

inline constexpr OptionDesc tcl_source_opts[] = {
    {.name = "-encoding", .takes_value = true, .value_hint = "encoding"},
};

inline constexpr FormDesc tcl_source_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_source_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcl_source_cmd{
    .name = "source",
    .arity = {1, 2},
    .arg_patterns = tcl_source_args,
    .options = tcl_source_opts,
    .forms = tcl_source_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_source_effects,
    .hover_index = 243,
};

// split
inline constexpr FormDesc tcl_split_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_split_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_split_cmd{
    .name = "split",
    .arity = {1, 2},
    .forms = tcl_split_forms,
    .pure = true,
    .cse_candidate = true,
    .return_type = TclType::LIST,
    .side_effect_hints = tcl_split_effects,
    .hover_index = 244,
};

// string
inline constexpr FormDesc tcl_string_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_string_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr OptionDesc tcl_string_sub_compare_opts[] = {
    {.name = "-nocase"},
    {.name = "-length", .takes_value = true, .value_hint = "int"},
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc tcl_string_sub_equal_opts[] = {
    {.name = "-nocase"},
    {.name = "-length", .takes_value = true, .value_hint = "int"},
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc tcl_string_sub_match_opts[] = {
    {.name = "-nocase"},
    {.name = "--", .is_terminator = true},
};

inline constexpr SubCmdDesc tcl_string_subcmds[] = {
    {.name = "cat", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .pure = true, .hover_index = 245},
    {.name = "compare", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .options = tcl_string_sub_compare_opts, .pure = true, .return_type = TclType::INT, .hover_index = 246},
    {.name = "equal", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .options = tcl_string_sub_equal_opts, .pure = true, .return_type = TclType::BOOLEAN, .hover_index = 247},
    {.name = "first", .arity = {2, 3}, .pure = true, .return_type = TclType::INT, .hover_index = 248},
    {.name = "index", .arity = {2, 2}, .pure = true, .hover_index = 249},
    {.name = "insert", .arity = {3, 3}, .hover_index = 250},
    {.name = "is", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .pure = true, .return_type = TclType::BOOLEAN, .hover_index = 251},
    {.name = "last", .arity = {2, 3}, .pure = true, .return_type = TclType::INT, .hover_index = 252},
    {.name = "length", .arity = {1, 1}, .pure = true, .return_type = TclType::INT, .hover_index = 253},
    {.name = "map", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .pure = true, .hover_index = 254},
    {.name = "match", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .options = tcl_string_sub_match_opts, .pure = true, .return_type = TclType::BOOLEAN, .hover_index = 255},
    {.name = "range", .arity = {3, 3}, .pure = true, .hover_index = 256},
    {.name = "repeat", .arity = {2, 2}, .pure = true, .hover_index = 257},
    {.name = "replace", .arity = {3, 4}, .pure = true, .hover_index = 258},
    {.name = "reverse", .arity = {1, 1}, .pure = true, .hover_index = 259},
    {.name = "tolower", .arity = {1, 3}, .pure = true, .hover_index = 260},
    {.name = "totitle", .arity = {1, 3}, .pure = true, .hover_index = 261},
    {.name = "toupper", .arity = {1, 3}, .pure = true, .hover_index = 262},
    {.name = "trim", .arity = {1, 2}, .pure = true, .hover_index = 263},
    {.name = "trimleft", .arity = {1, 2}, .pure = true, .hover_index = 264},
    {.name = "trimright", .arity = {1, 2}, .pure = true, .hover_index = 265},
    {.name = "wordend", .arity = {2, 2}, .pure = true, .return_type = TclType::INT, .hover_index = 266},
    {.name = "wordstart", .arity = {2, 2}, .pure = true, .return_type = TclType::INT, .hover_index = 267},
};

inline constexpr CommandDesc tcl_string_cmd{
    .name = "string",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_string_subcmds,
    .forms = tcl_string_forms,
    .cse_candidate = true,
    .side_effect_hints = tcl_string_effects,
    .hover_index = 268,
};

// subst
inline constexpr OptionDesc tcl_subst_opts[] = {
    {.name = "-nobackslashes"},
    {.name = "-nocommands"},
    {.name = "-novariables"},
};

inline constexpr FormDesc tcl_subst_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_subst_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcl_subst_cmd{
    .name = "subst",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = tcl_subst_opts,
    .forms = tcl_subst_forms,
    .taint_sink = true,
    .side_effect_hints = tcl_subst_effects,
    .hover_index = 269,
};

// switch
inline constexpr OptionDesc tcl_switch_opts[] = {
    {.name = "-exact", .detail = "Exact string compare mode."},
    {.name = "-glob", .detail = "Glob pattern mode."},
    {.name = "-regexp", .detail = "Regular expression mode."},
    {.name = "-nocase", .detail = "Case-insensitive matching."},
    {.name = "-matchvar", .takes_value = true, .value_hint = "varName", .detail = "Store match in variable (regexp mode)."},
    {.name = "-indexvar", .takes_value = true, .value_hint = "varName", .detail = "Store match indices in variable (regexp mode)."},
    {.name = "--", .is_terminator = true, .detail = "End of options."},
};

inline constexpr FormDesc tcl_switch_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_switch_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_switch_cmd{
    .name = "switch",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .layout_resolver = LayoutResolver::SWITCH,
    .options = tcl_switch_opts,
    .forms = tcl_switch_forms,
    .is_control_flow = true,
    .never_inline_body = true,
    .side_effect_hints = tcl_switch_effects,
    .hover_index = 270,
};

// tailcall
inline constexpr ArgPattern tcl_tailcall_args[] = {
    {.kind = FIXED, .role = VALUE, .index = 0},
};

inline constexpr FormDesc tcl_tailcall_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_tailcall_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE},
};

inline constexpr CommandDesc tcl_tailcall_cmd{
    .name = "tailcall",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_tailcall_args,
    .forms = tcl_tailcall_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_tailcall_effects,
    .hover_index = 271,
};

// tcl_findLibrary
inline constexpr CommandDesc tcl_tcl_findLibrary_cmd{
    .name = "tcl_findLibrary",
    .arity = {5, 6},
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .hover_index = 272,
};

// tell
inline constexpr ArgPattern tcl_tell_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
};

inline constexpr FormDesc tcl_tell_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_tell_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true},
};

inline constexpr CommandDesc tcl_tell_cmd{
    .name = "tell",
    .arity = {1, 1},
    .arg_patterns = tcl_tell_args,
    .forms = tcl_tell_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .return_type = TclType::INT,
    .side_effect_hints = tcl_tell_effects,
    .hover_index = 273,
};

// throw
inline constexpr FormDesc tcl_throw_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_throw_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_throw_cmd{
    .name = "throw",
    .arity = {2, 2},
    .forms = tcl_throw_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_throw_effects,
    .hover_index = 274,
};

// time
inline constexpr ArgPattern tcl_time_args[] = {
    {.kind = FIXED, .role = BODY, .index = 0},
};

inline constexpr FormDesc tcl_time_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_time_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_time_cmd{
    .name = "time",
    .arity = {1, 2},
    .arg_patterns = tcl_time_args,
    .forms = tcl_time_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_time_effects,
    .hover_index = 275,
};

// trace
inline constexpr FormDesc tcl_trace_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_trace_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr SubCmdDesc tcl_trace_subcmds[] = {
    {.name = "add", .arity = {2, std::numeric_limits<std::int32_t>::max()}},
    {.name = "info", .arity = {1, 2}},
    {.name = "remove", .arity = {2, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcl_trace_cmd{
    .name = "trace",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcl_trace_subcmds,
    .forms = tcl_trace_forms,
    .creates_dynamic_barrier = true,
    .side_effect_hints = tcl_trace_effects,
    .hover_index = 276,
};

// try
inline constexpr ArgPattern tcl_try_args[] = {
    {.kind = FIXED, .role = BODY, .index = 0},
};

inline constexpr FormDesc tcl_try_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_try_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr std::string_view tcl_try_keywords[] = {
    "on",
    "trap",
    "finally",
};

inline constexpr CommandDesc tcl_try_cmd{
    .name = "try",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_try_args,
    .layout_resolver = LayoutResolver::TRY,
    .forms = tcl_try_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .is_control_flow = true,
    .never_inline_body = true,
    .side_effect_hints = tcl_try_effects,
    .hover_index = 277,
    .formatting_keywords = tcl_try_keywords,
};

// unknown
inline constexpr FormDesc tcl_unknown_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_unknown_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true},
};

inline constexpr CommandDesc tcl_unknown_cmd{
    .name = "unknown",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_unknown_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .creates_dynamic_barrier = true,
    .side_effect_hints = tcl_unknown_effects,
    .hover_index = 278,
};

// unload
inline constexpr OptionDesc tcl_unload_opts[] = {
    {.name = "-nocomplain", .detail = "Suppresses all error messages."},
    {.name = "-keeplibrary", .detail = "This switch will prevent unload from issuing the operating system call that will unload the library from the process."},
    {.name = "--", .is_terminator = true, .detail = "Marks the end of switches."},
};

inline constexpr FormDesc tcl_unload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_unload_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_unload_cmd{
    .name = "unload",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = tcl_unload_opts,
    .forms = tcl_unload_forms,
    .side_effect_hints = tcl_unload_effects,
    .hover_index = 279,
};

// unset
inline constexpr ArgPattern tcl_unset_args[] = {
    {.kind = TAIL, .role = VAR_NAME, .index = 0},
};

inline constexpr OptionDesc tcl_unset_opts[] = {
    {.name = "-nocomplain", .detail = "Suppress errors for non-existent variables."},
    {.name = "--", .is_terminator = true, .detail = "End of options."},
};

inline constexpr FormDesc tcl_unset_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_unset_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .writes = true},
};

inline constexpr CommandDesc tcl_unset_cmd{
    .name = "unset",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_unset_args,
    .options = tcl_unset_opts,
    .forms = tcl_unset_forms,
    .assigns_variable_at = 0,
    .side_effect_hints = tcl_unset_effects,
    .hover_index = 280,
};

// update
inline constexpr FormDesc tcl_update_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_update_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_update_cmd{
    .name = "update",
    .arity = {0, 1},
    .forms = tcl_update_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_update_effects,
    .hover_index = 281,
};

// uplevel
inline constexpr ArgPattern tcl_uplevel_args[] = {
    {.kind = TAIL, .role = BODY, .index = 0},
};

inline constexpr FormDesc tcl_uplevel_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_uplevel_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_uplevel_cmd{
    .name = "uplevel",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_uplevel_args,
    .forms = tcl_uplevel_forms,
    .creates_dynamic_barrier = true,
    .unsafe = true,
    .taint_sink = true,
    .xc_translatable = -1,
    .side_effect_hints = tcl_uplevel_effects,
    .taint_hints = {.safe_colour = TaintColour::TAINTED},
    .hover_index = 282,
};

// upvar
inline constexpr ArgPattern tcl_upvar_args[] = {
    {.kind = TAIL, .role = VAR_NAME, .index = 0},
};

inline constexpr FormDesc tcl_upvar_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_upvar_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .writes = true},
};

inline constexpr CommandDesc tcl_upvar_cmd{
    .name = "upvar",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_upvar_args,
    .forms = tcl_upvar_forms,
    .creates_dynamic_barrier = true,
    .xc_translatable = -1,
    .side_effect_hints = tcl_upvar_effects,
    .hover_index = 283,
};

// variable
inline constexpr ArgPattern tcl_variable_args[] = {
    {.kind = TAIL, .role = VAR_NAME, .index = 0},
};

inline constexpr FormDesc tcl_variable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_variable_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .writes = true},
};

inline constexpr CommandDesc tcl_variable_cmd{
    .name = "variable",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = tcl_variable_args,
    .forms = tcl_variable_forms,
    .side_effect_hints = tcl_variable_effects,
    .hover_index = 284,
};

// vwait
inline constexpr ArgPattern tcl_vwait_args[] = {
    {.kind = FIXED, .role = VAR_READ, .index = 0},
};

inline constexpr OptionDesc tcl_vwait_opts[] = {
    {.name = "--", .is_terminator = true, .detail = "Marks the end of options."},
    {.name = "-all", .detail = "All conditions for the wait operation must be met to complete the wait operation."},
    {.name = "-extended", .detail = "An extended result in list form is returned, see below for explanation."},
    {.name = "-nofileevents", .detail = "File events are not handled in the wait operation."},
    {.name = "-noidleevents", .detail = "Idle handlers are not invoked during the wait operation."},
    {.name = "-notimerevents", .detail = "Timer handlers are not serviced during the wait operation."},
    {.name = "-nowindowevents", .detail = "Events of the windowing system are not handled during the wait operation."},
    {.name = "-readable", .takes_value = true, .detail = "Channel must name a Tcl channel open for reading."},
    {.name = "-timeout", .takes_value = true, .detail = "The wait operation is constrained to milliseconds."},
    {.name = "-variable", .takes_value = true, .detail = "VarName must be the name of a global variable."},
    {.name = "-writable", .takes_value = true, .detail = "Channel must name a Tcl channel open for writing."},
};

inline constexpr FormDesc tcl_vwait_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_vwait_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcl_vwait_cmd{
    .name = "vwait",
    .arity = {1, 1},
    .arg_patterns = tcl_vwait_args,
    .options = tcl_vwait_opts,
    .forms = tcl_vwait_forms,
    .dialects = DialectFlags::EDA_CADENCE | DialectFlags::EXPECT | DialectFlags::IAPPS | DialectFlags::IAPPS | DialectFlags::TMSH | DialectFlags::EDA_QUARTUS | DialectFlags::EDA_MENTOR | DialectFlags::EDA_SYNOPSYS | DialectFlags::TCL84 | DialectFlags::TCL85 | DialectFlags::TCL86 | DialectFlags::TCL90 | DialectFlags::EDA_XILINX,
    .side_effect_hints = tcl_vwait_effects,
    .hover_index = 285,
};

// while
inline constexpr ArgPattern tcl_while_args[] = {
    {.kind = FIXED, .role = EXPR, .index = 0},
    {.kind = FIXED, .role = BODY, .index = 1},
};

inline constexpr ArgTypeDesc tcl_while_types[] = {
    {.index = 0, .expected = TclType::BOOLEAN, .shimmers = true},
};

inline constexpr FormDesc tcl_while_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_while_effects[] = {
    {.target = SideEffectTarget::UNKNOWN},
};

inline constexpr CommandDesc tcl_while_cmd{
    .name = "while",
    .arity = {2, 2},
    .arg_patterns = tcl_while_args,
    .forms = tcl_while_forms,
    .is_control_flow = true,
    .never_inline_body = true,
    .has_loop_body = true,
    .arg_types = tcl_while_types,
    .side_effect_hints = tcl_while_effects,
    .hover_index = 286,
};

// yield
inline constexpr FormDesc tcl_yield_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_yield_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE},
};

inline constexpr CommandDesc tcl_yield_cmd{
    .name = "yield",
    .arity = {0, 1},
    .forms = tcl_yield_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_yield_effects,
    .hover_index = 287,
};

// yieldto
inline constexpr FormDesc tcl_yieldto_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcl_yieldto_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE},
};

inline constexpr CommandDesc tcl_yieldto_cmd{
    .name = "yieldto",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcl_yieldto_forms,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
    .side_effect_hints = tcl_yieldto_effects,
    .hover_index = 288,
};
