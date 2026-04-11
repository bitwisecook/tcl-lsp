// Tcl WASM runtime — root module.
//
// Imports all sub-modules and uses comptime references to ensure
// the linker keeps their exports even though nothing in this file
// calls them directly.

const tcl_obj = @import("tcl_obj.zig");
const tcl_globals = @import("tcl_globals.zig");
const tcl_io = @import("tcl_io.zig");
const tcl_string = @import("tcl_string.zig");
const tcl_list = @import("tcl_list.zig");
const tcl_dict = @import("tcl_dict.zig");
const tcl_catch = @import("tcl_catch.zig");
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

pub const global_set = tcl_globals.global_set;
pub const global_get = tcl_globals.global_get;
pub const tcl_incr = tcl_globals.tcl_incr;

pub const puts = tcl_io.puts;

pub const append = tcl_string.append;
pub const string_length = tcl_string.string_length;
pub const string_index = tcl_string.string_index;
pub const string_range = tcl_string.string_range;
pub const string_compare = tcl_string.string_compare;
pub const string_equal = tcl_string.string_equal;
pub const string_match = tcl_string.string_match;
pub const string_map = tcl_string.string_map;
pub const string_trim = tcl_string.string_trim;
pub const string_first = tcl_string.string_first;
pub const string_last = tcl_string.string_last;
pub const string_toupper = tcl_string.string_toupper;
pub const string_tolower = tcl_string.string_tolower;
pub const string_reverse = tcl_string.string_reverse;
pub const string_repeat = tcl_string.string_repeat;
pub const string_replace = tcl_string.string_replace;

pub const list_length = tcl_list.list_length;
pub const lappend = tcl_list.lappend;
pub const list_index = tcl_list.list_index;
pub const list_range = tcl_list.list_range;
pub const list_sort = tcl_list.list_sort;
pub const list_search = tcl_list.list_search;

pub const dict_create = tcl_dict.dict_create;
pub const dict_get = tcl_dict.dict_get;
pub const dict_set = tcl_dict.dict_set;
pub const dict_exists = tcl_dict.dict_exists;
pub const dict_keys = tcl_dict.dict_keys;
pub const dict_values = tcl_dict.dict_values;
pub const dict_size = tcl_dict.dict_size;

pub const catch_enter = tcl_catch.catch_enter;
pub const catch_leave = tcl_catch.catch_leave;
pub const catch_result = tcl_catch.catch_result;
pub const catch_has_error = tcl_catch.catch_has_error;
pub const @"error" = tcl_catch.@"error";
pub const error_flag = &tcl_catch.error_flag;

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
    _ = &tcl_io.puts;
    // tcl_string exports
    _ = &tcl_string.append;
    _ = &tcl_string.string_compare;
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
    _ = &tcl_string.string_replace;
    _ = &tcl_string.string_is_integer;
    _ = &tcl_string.string_is_alpha;
    _ = &tcl_string.string_is_digit;
    _ = &tcl_string.string_is_space;
    _ = &tcl_string.concat;
    // tcl_list exports
    _ = &tcl_list.list_length;
    _ = &tcl_list.lappend;
    _ = &tcl_list.tcl_list;
    _ = &tcl_list.list_index;
    _ = &tcl_list.list_range;
    _ = &tcl_list.list_sort;
    _ = &tcl_list.list_search;
    // tcl_dict exports
    _ = &tcl_dict.dict_create;
    _ = &tcl_dict.dict_get;
    _ = &tcl_dict.dict_set;
    _ = &tcl_dict.dict_exists;
    _ = &tcl_dict.dict_keys;
    _ = &tcl_dict.dict_values;
    _ = &tcl_dict.dict_size;
    // tcl_catch exports
    _ = &tcl_catch.catch_enter;
    _ = &tcl_catch.catch_leave;
    _ = &tcl_catch.catch_result;
    _ = &tcl_catch.catch_has_error;
    _ = &tcl_catch.@"error";
    _ = &tcl_catch.format;
    _ = &tcl_catch.regexp;
    _ = &tcl_catch.open;
    _ = &tcl_catch.close;
    _ = &tcl_catch.read;
    _ = &tcl_catch.gets;
    // tcl_interp exports
    _ = &interp.tcl_eval;
}
