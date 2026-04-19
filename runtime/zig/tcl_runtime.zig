// Tcl WASM runtime — root module.
//
// Imports all sub-modules and uses comptime references to ensure
// the linker keeps their exports even though nothing in this file
// calls them directly.

const tcl_obj = @import("tcl_obj.zig");
const tcl_globals = @import("tcl_globals.zig");
const tcl_io = @import("tcl_io.zig");
const tcl_string = @import("tcl_string.zig");
const tcl_list_mod = @import("tcl_list.zig");
const tcl_dict = @import("tcl_dict.zig");
const tcl_catch = @import("tcl_catch.zig");
const tcl_frames = @import("tcl_frames.zig");
const tcl_procs = @import("tcl_procs.zig");
const tcl_cmd_info = @import("tcl_cmd_info.zig");
const tcl_clock = @import("tcl_clock.zig");
const tcl_array = @import("tcl_array.zig");
const tcl_diag = @import("tcl_diag.zig");
const tcl_stubs = @import("tcl_stubs.zig");
const tcl_io_stubs = @import("tcl_io_stubs.zig");
const tcl_fs_stubs = @import("tcl_fs_stubs.zig");
const tcl_fmt_stubs = @import("tcl_fmt_stubs.zig");
const tcl_regex = @import("tcl_regex.zig");
const tcl_time_stubs = @import("tcl_time_stubs.zig");
const tcl_env_stubs = @import("tcl_env_stubs.zig");
const tcl_encoding = @import("tcl_encoding.zig");
const tcl_chan = @import("tcl_chan.zig");
const tcl_trace = @import("tcl_trace.zig");
const tcl_fs = @import("tcl_fs.zig");
const tcl_format = @import("tcl_format.zig");
const tcl_dispatch = @import("tcl_dispatch.zig");
const tcl_cmd_dispatch = @import("tcl_cmd_dispatch.zig");
const interp = @import("tcl_interp.zig");

// Re-export everything that tcl_interp.zig and other consumers need
// (backwards-compatible: code that does @import("tcl_runtime.zig").X still works)
pub const alloc = tcl_obj.alloc;
pub const memcpy = tcl_obj.memcpy;
pub const obj_new_string = tcl_obj.obj_new_string;
pub const obj_new_int = tcl_obj.obj_new_int;
pub const obj_get_int = tcl_obj.obj_get_int;
pub const obj_new_string_copy = tcl_obj.obj_new_string_copy;
pub const obj_ensure_string = tcl_obj.obj_ensure_string;
pub const list_count_elements = tcl_obj.list_count_elements;
pub const list_element_at = tcl_obj.list_element_at;
pub const copy_unbraced_elem = tcl_obj.copy_unbraced_elem;

pub const global_set = tcl_globals.global_set;
pub const global_get = tcl_globals.global_get;
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
pub const error_flag = &tcl_catch.error_flag;
pub const return_flag = &tcl_catch.return_flag;
pub const return_val = &tcl_catch.return_val;
pub const break_flag = &tcl_catch.break_flag;
pub const continue_flag = &tcl_catch.continue_flag;

// Frames
pub const frame_push = tcl_frames.frame_push;
pub const frame_pop = tcl_frames.frame_pop;
pub const frame_alias_global = tcl_frames.frame_alias_global;
pub const frame_depth_stash = tcl_frames.frame_depth_stash;
pub const frame_depth_restore = tcl_frames.frame_depth_restore;
pub const var_resolve = tcl_frames.var_resolve;
pub const var_set = tcl_frames.var_set;
pub const var_exists = tcl_frames.var_exists;
pub const local_set = tcl_frames.local_set;
pub const local_get = tcl_frames.local_get;

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
    _ = &tcl_globals.global_exists;
    _ = &tcl_globals.tcl_incr;
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
    _ = &tcl_catch.tcl_cmd_error;
    // tcl_*_stubs exports — stubs trap with ``unsupported command:
    // <name>`` so the compiled code sees a clear error rather than
    // a silent wrong answer.  Imports of these are wired up in
    // core/compiler/codegen/wasm.py's ``_RUNTIME_IMPORTS``; the
    // comptime references here ensure the linker keeps them.
    _ = &tcl_io_stubs.tcl_cmd_open;
    _ = &tcl_io_stubs.tcl_cmd_close;
    _ = &tcl_io_stubs.tcl_cmd_read;
    _ = &tcl_io_stubs.tcl_cmd_gets;
    _ = &tcl_io_stubs.tcl_cmd_eof;
    _ = &tcl_io_stubs.tcl_cmd_flush;
    _ = &tcl_io_stubs.tcl_cmd_fblocked;
    _ = &tcl_io_stubs.tcl_cmd_tell;
    _ = &tcl_io_stubs.tcl_cmd_seek;
    _ = &tcl_io_stubs.tcl_cmd_chan;
    _ = &tcl_io_stubs.tcl_cmd_fcopy;
    _ = &tcl_io_stubs.tcl_cmd_fileevent;
    _ = &tcl_io_stubs.tcl_cmd_socket;
    // file has a real impl in tcl_fs.zig.
    _ = &tcl_fs.tcl_cmd_file;
    _ = &tcl_fs_stubs.tcl_cmd_glob;
    // pwd and cd live in tcl_fs.zig (pass-through impl).
    _ = &tcl_fs.tcl_cmd_pwd;
    _ = &tcl_fs.tcl_cmd_cd;
    _ = &tcl_fs_stubs.tcl_cmd_exec;
    _ = &tcl_fs_stubs.tcl_cmd_source;
    _ = &tcl_fs_stubs.tcl_cmd_load;
    _ = &tcl_fs_stubs.tcl_cmd_unload;
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
    _ = &tcl_time_stubs.clock_format;
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
    _ = &tcl_cmd_dispatch.try_stub;
    _ = &tcl_dispatch.dispatch;
    // tcl_frames exports
    _ = &tcl_frames.frame_push;
    _ = &tcl_frames.frame_pop;
    _ = &tcl_frames.frame_get_depth;
    _ = &tcl_frames.frame_depth_stash;
    _ = &tcl_frames.frame_depth_restore;
    _ = &tcl_frames.local_set;
    _ = &tcl_frames.local_get;
    _ = &tcl_frames.local_exists;
    _ = &tcl_frames.var_resolve;
    _ = &tcl_frames.var_set;
    _ = &tcl_frames.var_exists;
    // tcl_procs exports
    _ = &tcl_procs.proc_register;
    _ = &tcl_procs.proc_register_compiled;
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
    // tcl_interp exports
    _ = &interp.tcl_eval;
    _ = &interp.ns_set;
    _ = &interp.ns_restore;
}
