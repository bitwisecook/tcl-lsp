// ── expect commands ──

// close
inline constexpr ArgPattern expect_close_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
};

inline constexpr OptionDesc expect_close_opts[] = {
    {.name = "-slave", .detail = "Close the slave side of the pty."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Close the specified spawn id."},
};

inline constexpr FormDesc expect_close_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_close_cmd{
    .name = "close",
    .arg_patterns = expect_close_args,
    .options = expect_close_opts,
    .forms = expect_close_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 602,
};

// debug
inline constexpr OptionDesc expect_debug_opts[] = {
    {.name = "-now", .detail = "Enter debugger immediately."},
};

inline constexpr FormDesc expect_debug_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_debug_cmd{
    .name = "debug",
    .options = expect_debug_opts,
    .forms = expect_debug_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 603,
};

// disconnect
inline constexpr FormDesc expect_disconnect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_disconnect_cmd{
    .name = "disconnect",
    .arity = {0, 0},
    .forms = expect_disconnect_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 604,
};

// exit
inline constexpr OptionDesc expect_exit_opts[] = {
    {.name = "-onexit", .takes_value = true, .value_hint = "command", .detail = "Register a handler to run at exit."},
    {.name = "-noexit", .detail = "Prepare for exit without exiting."},
};

inline constexpr FormDesc expect_exit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_exit_cmd{
    .name = "exit",
    .options = expect_exit_opts,
    .forms = expect_exit_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 605,
};

// exp_continue
inline constexpr OptionDesc expect_exp_continue_opts[] = {
    {.name = "-continue_timer", .detail = "Do not restart the timeout timer."},
};

inline constexpr FormDesc expect_exp_continue_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_exp_continue_cmd{
    .name = "exp_continue",
    .arity = {0, 1},
    .options = expect_exp_continue_opts,
    .forms = expect_exp_continue_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 606,
};

// exp_internal
inline constexpr OptionDesc expect_exp_internal_opts[] = {
    {.name = "-f", .takes_value = true, .value_hint = "file", .detail = "Log diagnostics to the specified file."},
};

inline constexpr FormDesc expect_exp_internal_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_exp_internal_cmd{
    .name = "exp_internal",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = expect_exp_internal_opts,
    .forms = expect_exp_internal_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 607,
};

// exp_pid
inline constexpr OptionDesc expect_exp_pid_opts[] = {
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Query the specified spawn id."},
};

inline constexpr FormDesc expect_exp_pid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_exp_pid_cmd{
    .name = "exp_pid",
    .options = expect_exp_pid_opts,
    .forms = expect_exp_pid_forms,
    .dialects = DialectFlags::EXPECT,
    .pure = true,
    .hover_index = 608,
};

// exp_version
inline constexpr FormDesc expect_exp_version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_exp_version_cmd{
    .name = "exp_version",
    .arity = {0, 1},
    .forms = expect_exp_version_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 609,
};

// expect
inline constexpr OptionDesc expect_expect_opts[] = {
    {.name = "-re", .detail = "Match pattern as a Tcl regular expression."},
    {.name = "-ex", .detail = "Match pattern as an exact string."},
    {.name = "-gl", .detail = "Match pattern as a glob (default)."},
    {.name = "-nocase", .detail = "Case-insensitive matching."},
    {.name = "-timeout", .takes_value = true, .value_hint = "seconds", .detail = "Override the timeout for this expect."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Specify the spawn id to expect from."},
    {.name = "-indices", .detail = "Store match indices in expect_out."},
    {.name = "-notransfer", .detail = "Do not consume matched output."},
};

inline constexpr FormDesc expect_expect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_expect_cmd{
    .name = "expect",
    .layout_resolver = LayoutResolver::EXPECT,
    .options = expect_expect_opts,
    .forms = expect_expect_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 610,
};

// expect_after
inline constexpr OptionDesc expect_expect_after_opts[] = {
    {.name = "-re", .detail = "Match as regular expression."},
    {.name = "-ex", .detail = "Match as exact string."},
    {.name = "-gl", .detail = "Match as glob (default)."},
    {.name = "-nocase", .detail = "Case-insensitive matching."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Specify the spawn id."},
    {.name = "-info", .detail = "Return current patterns."},
};

inline constexpr FormDesc expect_expect_after_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_expect_after_cmd{
    .name = "expect_after",
    .layout_resolver = LayoutResolver::EXPECT,
    .options = expect_expect_after_opts,
    .forms = expect_expect_after_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 611,
};

// expect_background
inline constexpr OptionDesc expect_expect_background_opts[] = {
    {.name = "-re", .detail = "Match as regular expression."},
    {.name = "-ex", .detail = "Match as exact string."},
    {.name = "-gl", .detail = "Match as glob (default)."},
    {.name = "-nocase", .detail = "Case-insensitive matching."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Specify the spawn id."},
};

inline constexpr FormDesc expect_expect_background_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_expect_background_cmd{
    .name = "expect_background",
    .layout_resolver = LayoutResolver::EXPECT,
    .options = expect_expect_background_opts,
    .forms = expect_expect_background_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 612,
};

// expect_before
inline constexpr OptionDesc expect_expect_before_opts[] = {
    {.name = "-re", .detail = "Match as regular expression."},
    {.name = "-ex", .detail = "Match as exact string."},
    {.name = "-gl", .detail = "Match as glob (default)."},
    {.name = "-nocase", .detail = "Case-insensitive matching."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Specify the spawn id."},
    {.name = "-info", .detail = "Return current patterns."},
};

inline constexpr FormDesc expect_expect_before_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_expect_before_cmd{
    .name = "expect_before",
    .layout_resolver = LayoutResolver::EXPECT,
    .options = expect_expect_before_opts,
    .forms = expect_expect_before_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 613,
};

// expect_tty
inline constexpr OptionDesc expect_expect_tty_opts[] = {
    {.name = "-re", .detail = "Match as regular expression."},
    {.name = "-ex", .detail = "Match as exact string."},
    {.name = "-gl", .detail = "Match as glob (default)."},
    {.name = "-nocase", .detail = "Case-insensitive matching."},
};

inline constexpr FormDesc expect_expect_tty_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_expect_tty_cmd{
    .name = "expect_tty",
    .layout_resolver = LayoutResolver::EXPECT,
    .options = expect_expect_tty_opts,
    .forms = expect_expect_tty_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 614,
};

// expect_user
inline constexpr OptionDesc expect_expect_user_opts[] = {
    {.name = "-re", .detail = "Match as regular expression."},
    {.name = "-ex", .detail = "Match as exact string."},
    {.name = "-gl", .detail = "Match as glob (default)."},
    {.name = "-nocase", .detail = "Case-insensitive matching."},
    {.name = "-timeout", .takes_value = true, .value_hint = "seconds", .detail = "Override the timeout."},
    {.name = "-indices", .detail = "Store match indices."},
    {.name = "-notransfer", .detail = "Do not consume matched output."},
};

inline constexpr FormDesc expect_expect_user_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_expect_user_cmd{
    .name = "expect_user",
    .layout_resolver = LayoutResolver::EXPECT,
    .options = expect_expect_user_opts,
    .forms = expect_expect_user_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 615,
};

// fork
inline constexpr FormDesc expect_fork_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_fork_cmd{
    .name = "fork",
    .arity = {0, 0},
    .forms = expect_fork_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 616,
};

// interact
inline constexpr OptionDesc expect_interact_opts[] = {
    {.name = "-re", .detail = "Match as regular expression."},
    {.name = "-ex", .detail = "Match as exact string."},
    {.name = "-input", .takes_value = true, .value_hint = "spawn_id", .detail = "Specify input source."},
    {.name = "-output", .takes_value = true, .value_hint = "spawn_id", .detail = "Specify output destination."},
    {.name = "-u", .takes_value = true, .value_hint = "spawn_id", .detail = "Connect user to the specified process."},
    {.name = "-o", .detail = "Apply to output."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Specify spawn id."},
    {.name = "-echo", .detail = "Echo characters."},
    {.name = "-nobuffer", .detail = "Do not buffer input."},
    {.name = "-f", .detail = "Force — do not flush."},
    {.name = "-F", .detail = "Force — flush."},
    {.name = "-reset", .detail = "Reset terminal modes."},
};

inline constexpr FormDesc expect_interact_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_interact_cmd{
    .name = "interact",
    .options = expect_interact_opts,
    .forms = expect_interact_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 617,
};

// log_file
inline constexpr OptionDesc expect_log_file_opts[] = {
    {.name = "-a", .detail = "Append to existing log file."},
    {.name = "-noappend", .detail = "Overwrite existing log file."},
    {.name = "-open", .takes_value = true, .value_hint = "fileId", .detail = "Log to an already-open Tcl file id."},
    {.name = "-leaveopen", .detail = "Leave the file open on close."},
    {.name = "-info", .detail = "Return current log file settings."},
};

inline constexpr FormDesc expect_log_file_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_log_file_cmd{
    .name = "log_file",
    .options = expect_log_file_opts,
    .forms = expect_log_file_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 618,
};

// log_user
inline constexpr OptionDesc expect_log_user_opts[] = {
    {.name = "-info", .detail = "Return current setting."},
};

inline constexpr FormDesc expect_log_user_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_log_user_cmd{
    .name = "log_user",
    .arity = {0, 1},
    .options = expect_log_user_opts,
    .forms = expect_log_user_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 619,
};

// match_max
inline constexpr OptionDesc expect_match_max_opts[] = {
    {.name = "-d", .detail = "Set the default for all spawn ids."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Set for the specified spawn id."},
};

inline constexpr FormDesc expect_match_max_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_match_max_cmd{
    .name = "match_max",
    .options = expect_match_max_opts,
    .forms = expect_match_max_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 620,
};

// overlay
inline constexpr FormDesc expect_overlay_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_overlay_cmd{
    .name = "overlay",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = expect_overlay_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 621,
};

// parity
inline constexpr OptionDesc expect_parity_opts[] = {
    {.name = "-d", .detail = "Set the default."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Set for the specified spawn id."},
};

inline constexpr FormDesc expect_parity_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_parity_cmd{
    .name = "parity",
    .options = expect_parity_opts,
    .forms = expect_parity_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 622,
};

// remove_nulls
inline constexpr OptionDesc expect_remove_nulls_opts[] = {
    {.name = "-d", .detail = "Set the default."},
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Set for the specified spawn id."},
};

inline constexpr FormDesc expect_remove_nulls_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_remove_nulls_cmd{
    .name = "remove_nulls",
    .options = expect_remove_nulls_opts,
    .forms = expect_remove_nulls_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 623,
};

// send
inline constexpr OptionDesc expect_send_opts[] = {
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Send to the specified spawn id."},
    {.name = "-raw", .detail = "Send without any translation."},
    {.name = "-null", .detail = "Send null characters."},
    {.name = "-break", .detail = "Send a break condition."},
    {.name = "-s", .detail = "Send slowly (obey send_slow parameters)."},
    {.name = "-h", .detail = "Send as if a human were typing (obey send_human parameters)."},
    {.name = "--", .is_terminator = true, .detail = "End of options."},
};

inline constexpr FormDesc expect_send_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_send_cmd{
    .name = "send",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = expect_send_opts,
    .forms = expect_send_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 624,
};

// send_error
inline constexpr OptionDesc expect_send_error_opts[] = {
    {.name = "-raw", .detail = "Send without translation."},
    {.name = "--", .is_terminator = true, .detail = "End of options."},
};

inline constexpr FormDesc expect_send_error_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_send_error_cmd{
    .name = "send_error",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = expect_send_error_opts,
    .forms = expect_send_error_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 625,
};

// send_log
inline constexpr OptionDesc expect_send_log_opts[] = {
    {.name = "--", .is_terminator = true, .detail = "End of options."},
};

inline constexpr FormDesc expect_send_log_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_send_log_cmd{
    .name = "send_log",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = expect_send_log_opts,
    .forms = expect_send_log_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 626,
};

// send_tty
inline constexpr OptionDesc expect_send_tty_opts[] = {
    {.name = "-raw", .detail = "Send without translation."},
    {.name = "--", .is_terminator = true, .detail = "End of options."},
};

inline constexpr FormDesc expect_send_tty_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_send_tty_cmd{
    .name = "send_tty",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = expect_send_tty_opts,
    .forms = expect_send_tty_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 627,
};

// send_user
inline constexpr OptionDesc expect_send_user_opts[] = {
    {.name = "-raw", .detail = "Send without translation."},
    {.name = "--", .is_terminator = true, .detail = "End of options."},
};

inline constexpr FormDesc expect_send_user_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_send_user_cmd{
    .name = "send_user",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = expect_send_user_opts,
    .forms = expect_send_user_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 628,
};

// sleep
inline constexpr FormDesc expect_sleep_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_sleep_cmd{
    .name = "sleep",
    .arity = {1, 1},
    .forms = expect_sleep_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 629,
};

// spawn
inline constexpr OptionDesc expect_spawn_opts[] = {
    {.name = "-noecho", .detail = "Suppress echoing of the command."},
    {.name = "-console", .detail = "Redirect console output to spawn."},
    {.name = "-ignore", .takes_value = true, .value_hint = "signal", .detail = "Ignore the named signal in the spawned process."},
    {.name = "-leaveopen", .detail = "Leave the file descriptor open."},
    {.name = "-pty", .detail = "Open a pty for the process."},
    {.name = "-nottycopy", .detail = "Do not copy tty modes."},
    {.name = "-nottyinit", .detail = "Do not initialise the tty."},
    {.name = "-open", .takes_value = true, .value_hint = "fileId", .detail = "Use an already-open file id."},
    {.name = "-trap", .detail = "Enable signal trapping."},
};

inline constexpr FormDesc expect_spawn_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_spawn_cmd{
    .name = "spawn",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = expect_spawn_opts,
    .forms = expect_spawn_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 630,
};

// strace
inline constexpr FormDesc expect_strace_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_strace_cmd{
    .name = "strace",
    .arity = {1, 1},
    .forms = expect_strace_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 631,
};

// stty
inline constexpr FormDesc expect_stty_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_stty_cmd{
    .name = "stty",
    .forms = expect_stty_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 632,
};

// system
inline constexpr FormDesc expect_system_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_system_cmd{
    .name = "system",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = expect_system_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 633,
};

// timestamp
inline constexpr OptionDesc expect_timestamp_opts[] = {
    {.name = "-seconds", .takes_value = true, .value_hint = "N", .detail = "Specify epoch seconds instead of current time."},
    {.name = "-format", .takes_value = true, .value_hint = "fmt", .detail = "strftime-style format string."},
    {.name = "-gmt", .detail = "Use GMT instead of local time."},
};

inline constexpr FormDesc expect_timestamp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_timestamp_cmd{
    .name = "timestamp",
    .options = expect_timestamp_opts,
    .forms = expect_timestamp_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 634,
};

// trap
inline constexpr ArgPattern expect_trap_args[] = {
    {.kind = FIXED, .role = BODY, .index = 0},
};

inline constexpr FormDesc expect_trap_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_trap_cmd{
    .name = "trap",
    .arg_patterns = expect_trap_args,
    .forms = expect_trap_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 635,
};

// wait
inline constexpr OptionDesc expect_wait_opts[] = {
    {.name = "-i", .takes_value = true, .value_hint = "spawn_id", .detail = "Wait for the specified spawn id."},
    {.name = "-nowait", .detail = "Non-blocking wait."},
};

inline constexpr FormDesc expect_wait_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc expect_wait_cmd{
    .name = "wait",
    .options = expect_wait_opts,
    .forms = expect_wait_forms,
    .dialects = DialectFlags::EXPECT,
    .hover_index = 636,
};
