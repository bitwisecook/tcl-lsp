// Tcl WASM runtime — root module.
//
// Imports all sub-modules and uses comptime references to ensure
// the linker keeps their exports even though nothing in this file
// calls them directly.

const tcl_obj = @import("valtypes/tcl_obj.zig");
// ``tcl_globals.zig`` retired in P3.4 — its four exports moved
// into ``tcl_ns.zig``.  The ``tcl_globals`` alias is kept as a
// convenience for the re-export block below; new code should
// import ``tcl_ns`` directly.
const tcl_globals = @import("interp/tcl_ns.zig");
const tcl_io = @import("io/tcl_io.zig");
const tcl_string = @import("valtypes/tcl_string.zig");
const tcl_list_mod = @import("valtypes/tcl_list.zig");
const tcl_dict = @import("valtypes/tcl_dict.zig");
const tcl_catch = @import("interp/tcl_catch.zig");
const tcl_frames = @import("interp/tcl_frames.zig");
const tcl_procs = @import("interp/tcl_procs.zig");
const tcl_ns = @import("interp/tcl_ns.zig");
const parse_cache = @import("valtypes/parse_cache.zig");
const tcl_cmd_info = @import("cmds/tcl_cmd_info.zig");
const tcl_clock = @import("io/tcl_clock.zig");
const tcl_array = @import("valtypes/tcl_array.zig");
const tcl_diag = @import("dispatch/tcl_diag.zig");
const tcl_stubs = @import("stubs/tcl_stubs.zig");
const tcl_io_stubs = @import("stubs/tcl_io_stubs.zig");
const tcl_fs_stubs = @import("stubs/tcl_fs_stubs.zig");
const tcl_fmt_stubs = @import("stubs/tcl_fmt_stubs.zig");
const tcl_regex = @import("valtypes/tcl_regex.zig");
const tcl_time_stubs = @import("stubs/tcl_time_stubs.zig");
const tcl_env_stubs = @import("stubs/tcl_env_stubs.zig");
const tcl_encoding = @import("valtypes/tcl_encoding.zig");
const tcl_chan = @import("io/tcl_chan.zig");
const tcl_trace = @import("interp/tcl_trace.zig");
const tcl_fs = @import("io/tcl_fs.zig");
const tcl_format = @import("valtypes/tcl_format.zig");
const tcl_arith = @import("valtypes/tcl_arith.zig");
const tcl_dispatch = @import("dispatch/tcl_dispatch.zig");
const tcl_stub_fallback = @import("dispatch/tcl_stub_fallback.zig");
const tcl_rename = @import("cmds/tcl_rename.zig");
const tcl_alias = @import("cmds/tcl_alias.zig");
const tcl_hide = @import("cmds/tcl_hide.zig");
const tcl_interp_registry = @import("interp/tcl_interp_registry.zig");
const interp = @import("interp/tcl_interp.zig");
const tcl_sched = @import("sched/tcl_sched.zig");
const tcl_coro = @import("sched/tcl_coro.zig");
const tcl_async = @import("sched/tcl_asyncify.zig");
const tcl_caps = @import("interp/tcl_caps.zig");
const tcl_cmd_exit = @import("cmds/exit.zig");

// Re-export everything that tcl_interp.zig and other consumers need
// (backwards-compatible: code that does @import("tcl_runtime.zig").X still works)
pub const alloc = tcl_obj.alloc;
pub const memcpy = tcl_obj.memcpy;
pub const obj_new_string = tcl_obj.obj_new_string;
pub const obj_new_int = tcl_obj.obj_new_int;
pub const obj_get_int = tcl_obj.obj_get_int;
pub const obj_new_float = tcl_obj.obj_new_float;
pub const obj_get_float = tcl_obj.obj_get_float;
pub const obj_new_string_copy = tcl_obj.obj_new_string_copy;
pub const obj_new_string_take = tcl_obj.obj_new_string_take;
pub const obj_ensure_string = tcl_obj.obj_ensure_string;
pub const list_count_elements = tcl_obj.list_count_elements;
pub const list_element_at = tcl_obj.list_element_at;
pub const copy_unbraced_elem = tcl_obj.copy_unbraced_elem;

pub const global_set = tcl_globals.global_set;
pub const global_get = tcl_globals.global_get;
pub const global_get_or_error = tcl_globals.global_get_or_error;
pub const global_exists = tcl_globals.global_exists;
pub const tcl_incr = tcl_globals.tcl_incr;

pub const tcl_cmd_puts = tcl_io.tcl_cmd_puts;
pub const tcl_cmd_puts_nonewline = tcl_io.tcl_cmd_puts_nonewline;

pub const tcl_cmd_append = tcl_string.tcl_cmd_append;
pub const string_length = tcl_string.string_length;
pub const string_index = tcl_string.string_index;
pub const string_range = tcl_string.string_range;
pub const string_compare = tcl_string.string_compare;
pub const tcl_expr_order_cmp = tcl_string.tcl_expr_order_cmp;
pub const string_equal = tcl_string.string_equal;
pub const string_match = tcl_string.string_match;
pub const string_map = tcl_string.string_map;
pub const string_map_nocase = tcl_string.string_map_nocase;
pub const string_trim = tcl_string.string_trim;
pub const string_trimleft = tcl_string.string_trimleft;
pub const string_trimright = tcl_string.string_trimright;
pub const string_first = tcl_string.string_first;
pub const string_last = tcl_string.string_last;
pub const string_toupper = tcl_string.string_toupper;
pub const string_tolower = tcl_string.string_tolower;
pub const string_totitle = tcl_string.string_totitle;
pub const string_reverse = tcl_string.string_reverse;
pub const string_repeat = tcl_string.string_repeat;
pub const string_replace = tcl_string.string_replace;
pub const tcl_cmd_split = tcl_string.tcl_cmd_split;
pub const tcl_cmd_join = tcl_string.tcl_cmd_join;
pub const tcl_cmd_concat = tcl_string.tcl_cmd_concat;

pub const tcl_cmd_list_length = tcl_list_mod.tcl_cmd_list_length;
pub const tcl_cmd_lappend = tcl_list_mod.tcl_cmd_lappend;
pub const tcl_list = tcl_list_mod.tcl_list;
pub const tcl_cmd_list_index = tcl_list_mod.tcl_cmd_list_index;
pub const tcl_cmd_list_range = tcl_list_mod.tcl_cmd_list_range;
pub const tcl_cmd_list_sort = tcl_list_mod.tcl_cmd_list_sort;
pub const tcl_cmd_list_reverse = tcl_list_mod.tcl_cmd_list_reverse;
pub const tcl_cmd_list_repeat = tcl_list_mod.tcl_cmd_list_repeat;
pub const tcl_cmd_list_insert = tcl_list_mod.tcl_cmd_list_insert;
pub const tcl_cmd_list_replace = tcl_list_mod.tcl_cmd_list_replace;
pub const tcl_cmd_list_set = tcl_list_mod.tcl_cmd_list_set;
pub const tcl_cmd_list_contains = tcl_list_mod.tcl_cmd_list_contains;
pub const tcl_cmd_list_search = tcl_list_mod.tcl_cmd_list_search;

pub const dict_create = tcl_dict.dict_create;
pub const dict_get = tcl_dict.dict_get;
pub const dict_set = tcl_dict.dict_set;
pub const dict_unset = tcl_dict.dict_unset;
pub const dict_exists = tcl_dict.dict_exists;
pub const dict_keys = tcl_dict.dict_keys;
pub const dict_values = tcl_dict.dict_values;
pub const dict_size = tcl_dict.dict_size;
pub const dict_merge_pair = tcl_dict.dict_merge_pair;

pub const catch_enter = tcl_catch.catch_enter;
pub const catch_leave = tcl_catch.catch_leave;
pub const catch_result = tcl_catch.catch_result;
pub const catch_has_error = tcl_catch.catch_has_error;
pub const catch_set_ok_result = tcl_catch.catch_set_ok_result;
pub const tcl_cmd_error = tcl_catch.tcl_cmd_error;
pub const var_unset_error = tcl_catch.var_unset_error;
pub const error_flag = &tcl_catch.error_flag;
pub const return_flag = &tcl_catch.return_flag;
pub const return_val = &tcl_catch.return_val;
pub const break_flag = &tcl_catch.break_flag;
pub const continue_flag = &tcl_catch.continue_flag;
pub const flow_consume_break = tcl_catch.flow_consume_break;
pub const flow_consume_continue = tcl_catch.flow_consume_continue;
pub const flow_check_return = tcl_catch.flow_check_return;
pub const flow_take_return = tcl_catch.flow_take_return;

// Frames
pub const frame_push = tcl_frames.frame_push;
pub const frame_pop = tcl_frames.frame_pop;
pub const frame_set_argv = tcl_frames.frame_set_argv;
pub const frame_get_argv = tcl_frames.frame_get_argv;
pub const frame_set_pending_argv0 = tcl_frames.frame_set_pending_argv0;
pub const frame_take_pending_argv0 = tcl_frames.frame_take_pending_argv0;
pub const frame_alias_global = tcl_frames.frame_alias_global;
pub const frame_depth_stash = tcl_frames.frame_depth_stash;
pub const frame_depth_restore = tcl_frames.frame_depth_restore;
pub const var_resolve = tcl_frames.var_resolve;
pub const var_set = tcl_frames.var_set;
pub const var_exists = tcl_frames.var_exists;
pub const local_set = tcl_frames.local_set;
pub const local_get = tcl_frames.local_get;
pub const local_get_or_error = tcl_frames.local_get_or_error;

// Procs
pub const proc_register = tcl_procs.proc_register;
pub const proc_lookup = tcl_procs.proc_lookup;
pub const proc_get_func_idx = tcl_procs.proc_get_func_idx;
pub const proc_get_n_params = tcl_procs.proc_get_n_params;
pub const proc_get_params = tcl_procs.proc_get_params;
pub const proc_get_body = tcl_procs.proc_get_body;

// Info
pub const info_exists = tcl_cmd_info.info_exists;
pub const info_dispatch = tcl_cmd_info.info_dispatch;

// Clock
pub const clock_seconds = tcl_clock.clock_seconds;
pub const clock_clicks = tcl_clock.clock_clicks;
pub const clock_milliseconds = tcl_clock.clock_milliseconds;

// Diagnostic / source-location map
pub const diag_set = tcl_diag.diag_set;
pub const diag_set_eval_ctx = tcl_diag.diag_set_eval_ctx;

// Arrays
pub const array_set = tcl_array.array_set;
pub const array_get = tcl_array.array_get;
pub const array_exists = tcl_array.array_exists;
pub const array_element_exists = tcl_array.array_element_exists;
pub const array_size = tcl_array.array_size;
pub const array_unset = tcl_array.array_unset;
pub const array_unset_element = tcl_array.array_unset_element;
pub const array_names = tcl_array.array_names;

// Ensure linker keeps all exported functions from each module.
comptime {
    // tcl_obj exports
    _ = &tcl_obj.obj_new_int;
    _ = &tcl_obj.obj_new_string;
    _ = &tcl_obj.obj_get_int;
    _ = &tcl_obj.tcl_obj_retain;
    _ = &tcl_obj.tcl_obj_release;
    _ = &tcl_obj.tcl_var_set;
    _ = &tcl_obj.tcl_var_get;
    // tcl_globals exports
    _ = &tcl_globals.global_set;
    _ = &tcl_globals.global_get;
    _ = &tcl_globals.global_get_or_error;
    _ = &tcl_globals.global_exists;
    _ = &tcl_globals.tcl_incr;
    // tcl_ns exports (P1.2 — no in-tree caller yet; comptime ref
    // keeps wasm-ld from dropping the new symbols)
    _ = &tcl_ns.tcl_ns_root;
    _ = &tcl_ns.tcl_ns_lookup;
    _ = &tcl_ns.tcl_ns_create;
    // P1.4 — ns_resolve_qualified test scaffolding
    _ = &tcl_ns.tcl_ns_resolve_qualified;
    _ = &tcl_ns.tcl_ns_last_simple_ptr;
    _ = &tcl_ns.tcl_ns_last_simple_len;
    _ = &tcl_ns.tcl_ns_last_alt;
    _ = &tcl_ns.tcl_test_alloc;
    // P3.1 — Var struct + var_table helpers; no in-tree caller
    // until P3.2 forwards globals through them.  The comptime refs
    // keep them in the binary so the P1.4 test harness (or P3.x
    // tests) can reach them.
    _ = &tcl_ns.ns_var_find;
    _ = &tcl_ns.ns_var_create;
    _ = &tcl_ns.var_resolve_link;
    _ = &tcl_ns.var_get_scalar;
    _ = &tcl_ns.var_set_scalar;
    // P9.1 — parse cache; no in-tree caller until P9.2 wires
    // ``eval_script`` through it.  Comptime refs keep wasm-ld
    // from dropping the symbols.
    _ = &parse_cache.lookup;
    _ = &parse_cache.insert;
    _ = &parse_cache.invalidate_all;
    _ = &parse_cache.alloc_slab;
    _ = &parse_cache.command_record;
    _ = &parse_cache.token_area_start;
    _ = &parse_cache.token_at;
    _ = &parse_cache.slab_n_commands;
    // tcl_io exports
    _ = &tcl_io.tcl_cmd_puts;
    _ = &tcl_io.tcl_cmd_puts_nonewline;
    // tcl_string exports
    _ = &tcl_string.tcl_cmd_append;
    _ = &tcl_string.string_compare;
    _ = &tcl_string.tcl_expr_order_cmp;
    _ = &tcl_string.string_length;
    _ = &tcl_string.string_index;
    _ = &tcl_string.string_range;
    _ = &tcl_string.string_map;
    _ = &tcl_string.string_map_nocase;
    _ = &tcl_string.string_match;
    _ = &tcl_string.string_trim;
    _ = &tcl_string.string_trimleft;
    _ = &tcl_string.string_trimright;
    _ = &tcl_string.string_equal;
    _ = &tcl_string.string_first;
    _ = &tcl_string.string_last;
    _ = &tcl_string.string_repeat;
    _ = &tcl_string.string_reverse;
    _ = &tcl_string.string_toupper;
    _ = &tcl_string.string_tolower;
    _ = &tcl_string.string_totitle;
    _ = &tcl_string.string_replace;
    _ = &tcl_string.string_is_integer;
    _ = &tcl_string.string_is_wideinteger;
    _ = &tcl_string.string_is_alpha;
    _ = &tcl_string.string_is_digit;
    _ = &tcl_string.string_is_space;
    _ = &tcl_string.tcl_cmd_split;
    _ = &tcl_string.tcl_cmd_join;
    _ = &tcl_string.tcl_cmd_concat;
    // tcl_list exports
    _ = &tcl_list_mod.tcl_cmd_list_length;
    _ = &tcl_list_mod.tcl_cmd_lappend;
    _ = &tcl_list_mod.tcl_list;
    _ = &tcl_list_mod.tcl_cmd_list_index;
    _ = &tcl_list_mod.tcl_cmd_list_range;
    _ = &tcl_list_mod.tcl_cmd_list_sort;
    _ = &tcl_list_mod.tcl_cmd_list_reverse;
    _ = &tcl_list_mod.tcl_cmd_list_repeat;
    _ = &tcl_list_mod.tcl_cmd_list_insert;
    _ = &tcl_list_mod.tcl_cmd_list_replace;
    _ = &tcl_list_mod.tcl_cmd_list_contains;
    _ = &tcl_list_mod.tcl_cmd_list_search;
    // tcl_dict exports
    _ = &tcl_dict.dict_create;
    _ = &tcl_dict.dict_get;
    _ = &tcl_dict.dict_set;
    _ = &tcl_dict.dict_unset;
    _ = &tcl_dict.dict_merge_pair;
    _ = &tcl_dict.dict_exists;
    _ = &tcl_dict.dict_keys;
    _ = &tcl_dict.dict_values;
    _ = &tcl_dict.dict_size;
    // tcl_catch exports
    _ = &tcl_catch.catch_enter;
    _ = &tcl_catch.catch_leave;
    _ = &tcl_catch.catch_result;
    _ = &tcl_catch.catch_has_error;
    _ = &tcl_catch.catch_set_ok_result;
    _ = &tcl_catch.flow_consume_break;
    _ = &tcl_catch.flow_consume_continue;
    _ = &tcl_catch.flow_check_return;
    _ = &tcl_catch.flow_take_return;
    _ = &tcl_catch.tcl_cmd_error;
    _ = &tcl_catch.tcl_return_set;
    _ = &tcl_catch.var_unset_error;
    // tcl_*_stubs exports — stubs trap with ``unsupported command:
    // <name>`` so the compiled code sees a clear error rather than
    // a silent wrong answer.  Imports of these are wired up in
    // core/compiler/codegen/wasm.py's ``_RUNTIME_IMPORTS``; the
    // comptime references here ensure the linker keeps them.
    // Channel commands with real WASI-backed implementations live
    // in tcl_chan.zig; only flush / chan / fileevent / socket remain
    // stubs in tcl_io_stubs.zig.
    _ = &tcl_chan.tcl_cmd_open;
    _ = &tcl_chan.tcl_cmd_close;
    _ = &tcl_chan.tcl_cmd_read;
    _ = &tcl_chan.tcl_cmd_gets;
    _ = &tcl_chan.tcl_cmd_eof;
    _ = &tcl_chan.tcl_cmd_fblocked;
    _ = &tcl_chan.tcl_cmd_tell;
    _ = &tcl_chan.tcl_cmd_seek;
    _ = &tcl_chan.tcl_cmd_fcopy;
    _ = &tcl_chan.tcl_cmd_puts_chan;
    _ = &tcl_io_stubs.tcl_cmd_flush;
    _ = &tcl_io_stubs.tcl_cmd_fileevent;
    _ = &tcl_io_stubs.tcl_cmd_socket;
    // file has a real impl in tcl_fs.zig.
    _ = &tcl_fs.tcl_cmd_file;
    // ``glob`` and ``exec`` are capability-gated BUILTINs in
    // ``runtime/zig/cmds/fs.zig`` and ``runtime/zig/cmds/exec.zig``;
    // their Python registry specs deliberately omit a
    // ``wasm_runtime_import`` so every call routes through the
    // eval-fallback into the BUILTIN dispatcher.  No comptime ref
    // needed — the cmd-table BUILTINS slice keeps the registrations
    // alive transitively.
    // pwd and cd live in tcl_fs.zig (pass-through impl).
    _ = &tcl_fs.tcl_cmd_pwd;
    _ = &tcl_fs.tcl_cmd_cd;
    // ``exit`` is capability-gated; the BUILTIN registers via the
    // cmd-table.  The export below is exposed for embedders that
    // want to call ``exit`` directly without going through Tcl.
    _ = &tcl_cmd_exit.tcl_cmd_exit;
    _ = &tcl_fs.tcl_cmd_source;
    _ = &tcl_fs_stubs.tcl_cmd_load;
    _ = &tcl_fs_stubs.tcl_cmd_unload;
    // Capability bitset — ``tcl_set_capabilities`` is the host-side
    // entry point used by embedders to grant CAP_EXEC / CAP_EXIT /
    // CAP_FS_GLOB before driving ``tcl_eval``.
    _ = &tcl_caps.tcl_set_capabilities;
    _ = &tcl_caps.tcl_get_capabilities;
    // format lives in tcl_format.zig (real impl).
    _ = &tcl_format.tcl_cmd_format;
    _ = &tcl_fmt_stubs.tcl_cmd_scan;
    _ = &tcl_fmt_stubs.tcl_cmd_binary;
    // ``regexp`` is wired to the real Tcl regex engine in
    // tcl_regex.zig; ``regsub`` remains a trapping stub until
    // the substitution path is implemented.
    _ = &tcl_regex.tcl_cmd_regexp;
    _ = &tcl_fmt_stubs.tcl_cmd_regsub;
    // encoding lives in tcl_encoding.zig (real pass-through impl).
    _ = &tcl_encoding.tcl_cmd_encoding;
    // fconfigure lives in tcl_chan.zig (NOP).
    _ = &tcl_chan.tcl_cmd_fconfigure;
    _ = &tcl_clock.clock_format;
    _ = &tcl_clock.clock_format_tz;
    _ = &tcl_clock.clock_add_pair;
    _ = &tcl_clock.clock_scan_obj;
    _ = &tcl_time_stubs.clock_scan;
    _ = &tcl_time_stubs.clock_add;
    _ = &tcl_time_stubs.tcl_cmd_after;
    _ = &tcl_time_stubs.tcl_cmd_vwait;
    _ = &tcl_time_stubs.tcl_cmd_update;
    _ = &tcl_time_stubs.tcl_cmd_coroutine;
    _ = &tcl_time_stubs.tcl_cmd_yield;
    _ = &tcl_time_stubs.tcl_cmd_yieldto;
    _ = &tcl_env_stubs.@"namespace";
    _ = &tcl_env_stubs.tcl_cmd_package_cmd;
    // trace lives in tcl_trace.zig (pass-through impl).
    _ = &tcl_trace.tcl_cmd_trace_cmd;
    _ = &tcl_env_stubs.tcl_cmd_interp_cmd;
    _ = &tcl_env_stubs.tcl_cmd_apply;
    _ = &tcl_stubs.unsupported;
    _ = &tcl_stub_fallback.try_stub;
    _ = &tcl_dispatch.dispatch;
    // tcl_frames exports
    _ = &tcl_frames.frame_push;
    _ = &tcl_frames.frame_pop;
    _ = &tcl_frames.frame_set_argv;
    _ = &tcl_frames.frame_get_argv;
    _ = &tcl_frames.frame_set_pending_argv0;
    _ = &tcl_frames.frame_take_pending_argv0;
    _ = &tcl_frames.frame_get_depth;
    _ = &tcl_frames.frame_depth_stash;
    _ = &tcl_frames.frame_depth_restore;
    _ = &tcl_frames.local_set;
    _ = &tcl_frames.local_get;
    _ = &tcl_frames.local_get_or_error;
    _ = &tcl_frames.local_exists;
    _ = &tcl_frames.var_resolve;
    _ = &tcl_frames.var_set;
    _ = &tcl_frames.var_exists;
    // tcl_procs exports
    _ = &tcl_procs.proc_register;
    _ = &tcl_procs.proc_register_compiled;
    _ = &tcl_procs.proc_set_body_source;
    _ = &tcl_procs.proc_lookup;
    _ = &tcl_procs.proc_get_func_idx;
    _ = &tcl_procs.proc_get_n_params;
    _ = &tcl_procs.proc_get_args_tail;
    _ = &tcl_procs.proc_get_params;
    _ = &tcl_procs.proc_get_body;
    _ = &tcl_procs.proc_exists;
    // tcl_cmd_info exports
    _ = &tcl_cmd_info.info_exists;
    _ = &tcl_cmd_info.info_body;
    _ = &tcl_cmd_info.info_args;
    _ = &tcl_cmd_info.info_dispatch;
    // tcl_clock exports
    _ = &tcl_clock.clock_seconds;
    _ = &tcl_clock.clock_clicks;
    _ = &tcl_clock.clock_milliseconds;
    // tcl_diag exports
    _ = &tcl_diag.diag_set;
    _ = &tcl_diag.diag_set_eval_ctx;
    // tcl_array exports
    _ = &tcl_array.array_set;
    _ = &tcl_array.array_get;
    _ = &tcl_array.array_exists;
    _ = &tcl_array.array_element_exists;
    _ = &tcl_array.array_size;
    _ = &tcl_array.array_unset;
    _ = &tcl_array.array_unset_element;
    _ = &tcl_array.array_names;
    // tcl_arith exports — float-aware arithmetic helpers
    _ = &tcl_arith.tcl_arith_add;
    _ = &tcl_arith.tcl_arith_sub;
    _ = &tcl_arith.tcl_arith_mul;
    _ = &tcl_arith.tcl_arith_div;
    _ = &tcl_arith.tcl_arith_mod;
    _ = &tcl_arith.tcl_arith_lshift;
    _ = &tcl_arith.tcl_arith_rshift;
    _ = &tcl_arith.tcl_arith_band;
    _ = &tcl_arith.tcl_arith_bor;
    _ = &tcl_arith.tcl_arith_bxor;
    _ = &tcl_arith.tcl_arith_bnot;
    _ = &tcl_arith.tcl_arith_neg;
    _ = &tcl_arith.tcl_arith_pow;
    _ = &tcl_arith.tcl_math_double;
    _ = &tcl_arith.tcl_math_int;
    _ = &tcl_arith.tcl_math_round;
    _ = &tcl_arith.tcl_math_log;
    _ = &tcl_arith.tcl_math_sqrt;
    _ = &tcl_arith.tcl_math_exp;
    _ = &tcl_arith.tcl_math_log10;
    _ = &tcl_arith.tcl_math_sin;
    _ = &tcl_arith.tcl_math_cos;
    _ = &tcl_arith.tcl_math_fabs;
    _ = &tcl_arith.tcl_math_tan;
    _ = &tcl_arith.tcl_math_asin;
    _ = &tcl_arith.tcl_math_acos;
    _ = &tcl_arith.tcl_math_atan;
    _ = &tcl_arith.tcl_math_atan2;
    _ = &tcl_arith.tcl_math_sinh;
    _ = &tcl_arith.tcl_math_cosh;
    _ = &tcl_arith.tcl_math_tanh;
    _ = &tcl_arith.tcl_math_floor;
    _ = &tcl_arith.tcl_math_ceil;
    _ = &tcl_arith.tcl_math_fmod;
    _ = &tcl_arith.tcl_math_hypot;
    // obj float exports
    _ = &tcl_obj.obj_new_float;
    _ = &tcl_obj.obj_get_float;
    // tcl_interp exports
    _ = &interp.tcl_eval;
    _ = &interp.ns_set;
    _ = &interp.ns_restore;

    // tcl_rename / tcl_alias — new in the rename-alias wave.  No
    // direct WASM export today (all access goes through the
    // ``rename`` / ``interp alias`` built-ins in tcl_interp.zig and
    // the runtime test harness), but the comptime refs keep
    // wasm-ld from pruning symbols the tests reach through.
    _ = &tcl_rename.rename_command;
    _ = &tcl_alias.alias_alloc;
    _ = &tcl_alias.alias_rec;
    _ = &tcl_alias.alias_clear;
    _ = &tcl_alias.alias_describe;
    _ = &tcl_alias.is_alias;
    _ = &tcl_ns.ns_cmd_clear;
    // tcl_hide — new in the hide/introspection wave.  Built-ins
    // in tcl_interp.zig call these directly; test scaffolding
    // below exposes them to the Python harness too.
    _ = &tcl_hide.hide_command;
    _ = &tcl_hide.expose_command;
    _ = &tcl_interp_registry.hidden_put;
    _ = &tcl_interp_registry.hidden_find;
    _ = &tcl_interp_registry.hidden_clear;
    _ = &tcl_interp_registry.hidden_table_buf;
    _ = &tcl_interp_registry.hidden_table_cap;
    // tcl_interp_registry — child-interp wave.  Builtins dispatch
    // through these; tests drive them via scaffolding exports
    // declared at the bottom of this file.
    _ = &tcl_interp_registry.interp_root;
    _ = &tcl_interp_registry.interp_current;
    _ = &tcl_interp_registry.child_create;
    _ = &tcl_interp_registry.child_lookup;
    _ = &tcl_interp_registry.child_delete;
    _ = &tcl_interp_registry.resolve_path;
    _ = &tcl_interp_registry.enter;
    _ = &tcl_interp_registry.leave;
    // tcl_cmd_info — new walkers for info commands / info procs /
    // info default.  info_dispatch already re-exported above
    // picks up the new entry points transitively.
    _ = &tcl_cmd_info.info_commands;
    _ = &tcl_cmd_info.info_procs;
    _ = &tcl_cmd_info.info_default;
    // tcl_procs — sidecar accessor added for compiled-proc rename
    // completeness.
    _ = &tcl_procs.proc_get_export_name;
    // Runtime test scaffolding — exposed as WASM exports so the
    // Python test harness can drive primitives without going
    // through the full Tcl interpreter.
    _ = &tcl_test_rename;
    _ = &tcl_test_alias_create;
    _ = &tcl_test_hide;
    _ = &tcl_test_expose;
    _ = &tcl_test_info_commands;
    _ = &tcl_test_info_procs;
    _ = &tcl_test_namespace_which;
    _ = &tcl_test_hidden_exists;
    _ = &tcl_test_export_name_ptr;
    _ = &tcl_test_export_name_len;
    _ = &tcl_test_obj_str_ptr;
    _ = &tcl_test_obj_str_len;
    // Child-interp scaffolding.
    _ = &tcl_test_interp_root;
    _ = &tcl_test_interp_current;
    _ = &tcl_test_interp_create;
    _ = &tcl_test_interp_lookup;
    _ = &tcl_test_interp_delete;
    _ = &tcl_test_interp_eval_script;
    _ = &tcl_test_interp_root_ns;
    _ = &tcl_test_hidden_find_in;
    // Stage-2 coroutine driver — exported so wasm-opt --asyncify can
    // reference it by name in the removelist.  No host caller reaches
    // this directly today (the runtime drives coroutines via
    // ``eval_proc_call_bucket``); the comptime ref ensures the linker
    // keeps the symbol when the export attribute alone wouldn't (Zig
    // 0.16's wasm-ld otherwise prunes externally-unused exports
    // even with ``rdynamic``).
    _ = &tcl_coro.tcl_coro_drive;
}

// -- Runtime test scaffolding (rename / alias) -----------------------------
//
// Exposed as plain WASM exports so ``tests/runtime/test_tcl_rename.py``
// and ``tests/runtime/test_tcl_alias.py`` can drive the primitives
// directly.  Not emitted by the compiler — the i32 in / out shape
// matches the existing test exports (``tcl_test_alloc``,
// ``tcl_ns_resolve_qualified``, …).

/// Rename a command.  ``old_ns`` / ``new_ns`` are ``*Namespace``
/// handles; name spans are (ptr, len) into linear memory.
/// ``new_simple_len == 0`` is the delete form.  Returns the
/// ``RenameResult`` enum value as an i32 (0 = ok, 1 = not_found,
/// 2 = target_exists, 3 = builtin_protected).
pub export fn tcl_test_rename(
    old_ns: i32,
    old_simple_ptr: i32,
    old_simple_len: i32,
    new_ns: i32,
    new_simple_ptr: i32,
    new_simple_len: i32,
) i32 {
    const r = tcl_rename.rename_command(
        @bitCast(old_ns),
        @bitCast(old_simple_ptr),
        @bitCast(old_simple_len),
        @bitCast(new_ns),
        @bitCast(new_simple_ptr),
        @bitCast(new_simple_len),
    );
    return @bitCast(@as(u32, @intFromEnum(r)));
}

/// Allocate an alias redirect Command + insert it under ``dest_ns``.
/// Used by the runtime-test harness to construct aliases without
/// the full ``interp alias`` parser.  Returns the Command address.
pub export fn tcl_test_alias_create(
    dest_ns: i32,
    new_simple_ptr: i32,
    new_simple_len: i32,
    target_name_ptr: i32,
    target_name_len: i32,
    n_prefix: i32,
    prefix_args_addr: i32,
) i32 {
    const cmd = tcl_alias.alias_alloc(
        @bitCast(dest_ns),
        @bitCast(new_simple_ptr),
        @bitCast(new_simple_len),
        @bitCast(target_name_ptr),
        @bitCast(target_name_len),
        @bitCast(n_prefix),
        @bitCast(prefix_args_addr),
        0, // parent_interp — scaffolding always creates same-interp aliases
    );
    _ = tcl_ns.ns_cmd_put(
        @bitCast(dest_ns),
        @bitCast(new_simple_ptr),
        @bitCast(new_simple_len),
        cmd,
    );
    tcl_procs.proc_count_bump();
    return @bitCast(cmd);
}

// -- Runtime test scaffolding (hide / expose / info) -----------------------

/// Hide a command.  ``src_ns`` identifies the namespace holding
/// the source.  Post-child-interp the hidden table is keyed per
/// interp; this export routes to the current interp so existing
/// tests don't need to thread the handle.  Returns the ``HideResult``
/// enum as an i32 (0 = ok, 1 = not_found, 2 = qualified_name_rejected,
/// 3 = hidden_name_taken).
pub export fn tcl_test_hide(
    src_ns: i32,
    src_simple_ptr: i32,
    src_simple_len: i32,
    hidden_name_ptr: i32,
    hidden_name_len: i32,
) i32 {
    const r = tcl_hide.hide_command(
        tcl_interp_registry.interp_current(),
        @bitCast(src_ns),
        @bitCast(src_simple_ptr),
        @bitCast(src_simple_len),
        @bitCast(hidden_name_ptr),
        @bitCast(hidden_name_len),
    );
    return @bitCast(@as(u32, @intFromEnum(r)));
}

/// Expose a hidden command.  Returns the ``ExposeResult`` enum as
/// an i32 (0 = ok, 1 = not_found, 2 = qualified_name_rejected,
/// 3 = target_exists).
pub export fn tcl_test_expose(
    hidden_name_ptr: i32,
    hidden_name_len: i32,
    dest_ns: i32,
    new_simple_ptr: i32,
    new_simple_len: i32,
) i32 {
    const r = tcl_hide.expose_command(
        tcl_interp_registry.interp_current(),
        @bitCast(hidden_name_ptr),
        @bitCast(hidden_name_len),
        @bitCast(dest_ns),
        @bitCast(new_simple_ptr),
        @bitCast(new_simple_len),
    );
    return @bitCast(@as(u32, @intFromEnum(r)));
}

/// Non-zero if a hidden-table entry is present under the given
/// simple name.  Used by tests to assert the hidden state directly
/// without the interpreter layer.  Routes to the current interp;
/// :func:`tcl_test_hidden_find_in` takes an explicit ``Interp*``.
pub export fn tcl_test_hidden_exists(name_ptr: i32, name_len: i32) i32 {
    const h = tcl_interp_registry.hidden_find(
        tcl_interp_registry.interp_current(),
        @bitCast(name_ptr),
        @bitCast(name_len),
    );
    return if (h != 0) 1 else 0;
}

/// Run ``info commands pattern`` and return the result TclObj.
/// ``pattern_len == 0`` is the "no pattern" case.  Stages a TclObj
/// wrapping the supplied pattern bytes so the walker sees the same
/// shape it would via the interpreter dispatch.
pub export fn tcl_test_info_commands(pattern_ptr: i32, pattern_len: i32) i32 {
    if (pattern_len == 0) return tcl_cmd_info.info_commands(0);
    const pat = tcl_obj.obj_new_string(pattern_ptr, pattern_len);
    return tcl_cmd_info.info_commands(pat);
}

/// Run ``info procs pattern``.  Same shape as
/// :func:`tcl_test_info_commands`.
pub export fn tcl_test_info_procs(pattern_ptr: i32, pattern_len: i32) i32 {
    if (pattern_len == 0) return tcl_cmd_info.info_procs(0);
    const pat = tcl_obj.obj_new_string(pattern_ptr, pattern_len);
    return tcl_cmd_info.info_procs(pat);
}

/// Probe ``namespace which -command name`` without routing through
/// the interpreter.  Returns the resolved Command's stored FQN as
/// a TclObj, or an empty-string TclObj on miss.  The test harness
/// reads the TclObj's str_ptr / str_len slots to verify.
pub export fn tcl_test_namespace_which(name_ptr: i32, name_len: i32) i32 {
    const cmd = tcl_ns.ns_find_command(
        tcl_ns.ns_current(),
        @bitCast(name_ptr),
        @bitCast(name_len),
    );
    if (cmd == 0) return tcl_obj.obj_new_string(0, 0);
    const fp: i32 = tcl_obj.read_i32(cmd);
    const fl: i32 = tcl_obj.read_i32(cmd + 4);
    return tcl_obj.obj_new_string(fp, fl);
}

/// Return the sidecar export-name pointer stashed by
/// ``proc_register_compiled``.  Tests use this to verify the
/// ``OFF_EXPORT_NAME_BUCKET`` slot is populated correctly and
/// survives a rename.
pub export fn tcl_test_export_name_ptr(bucket: i32) i32 {
    const e = tcl_procs.proc_get_export_name(@bitCast(bucket));
    return @bitCast(e.ptr);
}

pub export fn tcl_test_export_name_len(bucket: i32) i32 {
    const e = tcl_procs.proc_get_export_name(@bitCast(bucket));
    return @bitCast(e.len);
}

/// Force the TclObj at ``obj`` to materialise its string rep, then
/// return the rep's pointer.  For int-shaped TclObjs the
/// ``str_ptr`` / ``str_len`` slots stay at zero until something
/// walks the string form; this export lets the Python test
/// harness read the decimal rendering of results like ``info
/// level`` / ``llength`` / ``lsearch -exact`` without relying on
/// compiler-side string interpolation to trigger materialisation.
/// Pairs with ``tcl_test_obj_str_len``.
pub export fn tcl_test_obj_str_ptr(obj: i32) i32 {
    const s = tcl_obj.obj_ensure_string(obj);
    return @bitCast(s.ptr);
}

pub export fn tcl_test_obj_str_len(obj: i32) i32 {
    const s = tcl_obj.obj_ensure_string(obj);
    return @bitCast(s.len);
}

// -- Runtime test scaffolding (child interps) ------------------------------

/// Return the root ``Interp*`` (allocating it on first call, same
/// shape as ``tcl_ns_root``).  Tests use this to anchor subsequent
/// ``tcl_test_interp_create`` calls.
pub export fn tcl_test_interp_root() i32 {
    return @bitCast(tcl_interp_registry.interp_root());
}

/// Return the currently-active ``Interp*``.  Tests call this around
/// ``interp eval`` to verify the swap / restore pair worked.
pub export fn tcl_test_interp_current() i32 {
    return @bitCast(tcl_interp_registry.interp_current());
}

/// Create a child interp under ``parent`` with the given simple
/// name.  Returns the new ``Interp*`` address.  ``flags`` packs
/// ``INTERP_SAFE`` (and future flag bits).
pub export fn tcl_test_interp_create(
    parent: i32,
    name_ptr: i32,
    name_len: i32,
    flags: i32,
) i32 {
    return @bitCast(tcl_interp_registry.child_create(
        @bitCast(parent),
        @bitCast(name_ptr),
        @bitCast(name_len),
        @bitCast(flags),
    ));
}

/// Look up a direct child of ``parent`` by simple name.  Returns
/// 0 on miss.
pub export fn tcl_test_interp_lookup(parent: i32, name_ptr: i32, name_len: i32) i32 {
    return @bitCast(tcl_interp_registry.child_lookup(
        @bitCast(parent),
        @bitCast(name_ptr),
        @bitCast(name_len),
    ));
}

/// Delete a child interp from its parent's registry.  Returns 1
/// if the entry was cleared, 0 otherwise.
pub export fn tcl_test_interp_delete(parent: i32, name_ptr: i32, name_len: i32) i32 {
    const ok = tcl_interp_registry.child_delete(
        @bitCast(parent),
        @bitCast(name_ptr),
        @bitCast(name_len),
    );
    return if (ok) 1 else 0;
}

/// Evaluate a script inside the target interp (no path-resolver
/// call: tests pass the pre-resolved ``Interp*`` directly).
/// Returns the TclObj result from ``eval_script``.
pub export fn tcl_test_interp_eval_script(
    target_interp: i32,
    script_ptr: i32,
    script_len: i32,
) i32 {
    const save = tcl_interp_registry.enter(@bitCast(target_interp));
    const res = interp.tcl_eval(tcl_obj.obj_new_string(script_ptr, script_len));
    tcl_interp_registry.leave(save);
    return res;
}

/// Return the target interp's root ``Namespace*``.  Tests use this
/// to register procs inside a child interp via ``proc_register``
/// after wrapping them in an ``enter`` / ``leave`` pair.
pub export fn tcl_test_interp_root_ns(target_interp: i32) i32 {
    const t: *tcl_interp_registry.Interp = @ptrFromInt(@as(u32, @bitCast(target_interp)));
    return @bitCast(t.root_ns);
}

/// Probe a specific interp's hidden table by simple name.  Returns
/// non-zero if an entry is present.
pub export fn tcl_test_hidden_find_in(
    target_interp: i32,
    name_ptr: i32,
    name_len: i32,
) i32 {
    const h = tcl_interp_registry.hidden_find(
        @bitCast(target_interp),
        @bitCast(name_ptr),
        @bitCast(name_len),
    );
    return if (h != 0) 1 else 0;
}
