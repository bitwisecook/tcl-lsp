"""Runtime import tables consumed by the WASM codegen.

Each table maps a compile-time key to the Zig-exported runtime
function (or a structured descriptor) the emitter calls into.
Separated from the emitter so import tables can be audited,
extended, and stubbed independently of codegen changes.
"""

from __future__ import annotations

from ._ir import ValType

# Runtime function signatures imported from the Tcl runtime.
# Each entry maps an import key to (module, export_name, param_types, result_types).
#
# Values are represented as i32 TclObj pointers.  The TclObj lifecycle
# imports (obj_new_int, obj_new_string, obj_get_int) bridge between raw
# WASM integers (i64) used in expr arithmetic and the i32 object pointers
# passed to/from runtime functions and stored in locals.
_RUNTIME_IMPORTS: dict[str, tuple[str, str, list[ValType], list[ValType]]] = {
    # TclObj lifecycle — always imported
    "tcl_obj_new_int": ("tcl", "obj_new_int", [ValType.I64], [ValType.I32]),
    "tcl_obj_new_string": ("tcl", "obj_new_string", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_obj_get_int": ("tcl", "obj_get_int", [ValType.I32], [ValType.I64]),
    # Command runtime — all parameters/results are i32 TclObj pointers
    "tcl_puts": ("tcl", "tcl_cmd_puts", [ValType.I32], [ValType.I32]),
    "tcl_puts_nonewline": (
        "tcl",
        "tcl_cmd_puts_nonewline",
        [ValType.I32],
        [ValType.I32],
    ),
    "tcl_append": ("tcl", "tcl_cmd_append", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_length": ("tcl", "tcl_cmd_list_length", [ValType.I32], [ValType.I32]),
    "tcl_lappend": ("tcl", "tcl_cmd_lappend", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_length": ("tcl", "string_length", [ValType.I32], [ValType.I32]),
    "tcl_string_index": ("tcl", "string_index", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_range": (
        "tcl",
        "string_range",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_compare": (
        "tcl",
        "string_compare",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_expr_order_cmp": (
        "tcl",
        "tcl_expr_order_cmp",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_map": ("tcl", "string_map", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_match": ("tcl", "string_match", [ValType.I32, ValType.I32], [ValType.I32]),
    # ``string trim{,left,right} value ?chars?`` — a null (i32 0) chars arg
    # means "default whitespace"; otherwise the TclObj's string value is the
    # set of bytes to trim.
    "tcl_string_trim": ("tcl", "string_trim", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_trimleft": (
        "tcl",
        "string_trimleft",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_trimright": (
        "tcl",
        "string_trimright",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_equal": ("tcl", "string_equal", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_first": ("tcl", "string_first", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_last": ("tcl", "string_last", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_repeat": ("tcl", "string_repeat", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_string_reverse": ("tcl", "string_reverse", [ValType.I32], [ValType.I32]),
    "tcl_string_toupper": ("tcl", "string_toupper", [ValType.I32], [ValType.I32]),
    "tcl_string_tolower": ("tcl", "string_tolower", [ValType.I32], [ValType.I32]),
    "tcl_string_totitle": ("tcl", "string_totitle", [ValType.I32], [ValType.I32]),
    "tcl_string_replace": (
        "tcl",
        "string_replace",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_string_is_integer": ("tcl", "string_is_integer", [ValType.I32], [ValType.I32]),
    "tcl_string_is_alpha": ("tcl", "string_is_alpha", [ValType.I32], [ValType.I32]),
    "tcl_string_is_digit": ("tcl", "string_is_digit", [ValType.I32], [ValType.I32]),
    "tcl_string_is_space": ("tcl", "string_is_space", [ValType.I32], [ValType.I32]),
    "tcl_list_create": ("tcl", "tcl_list", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_concat": ("tcl", "tcl_cmd_concat", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_index": ("tcl", "tcl_cmd_list_index", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_range": (
        "tcl",
        "tcl_cmd_list_range",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_tail": ("tcl", "list_tail", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_sort": ("tcl", "tcl_cmd_list_sort", [ValType.I32], [ValType.I32]),
    "tcl_list_reverse": ("tcl", "tcl_cmd_list_reverse", [ValType.I32], [ValType.I32]),
    "tcl_list_contains": (
        "tcl",
        "tcl_cmd_list_contains",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_repeat": (
        "tcl",
        "tcl_cmd_list_repeat",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_insert": (
        "tcl",
        "tcl_cmd_list_insert",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_replace": (
        "tcl",
        "tcl_cmd_list_replace",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_set": (
        "tcl",
        "tcl_cmd_list_set",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_search": ("tcl", "tcl_cmd_list_search", [ValType.I32, ValType.I32], [ValType.I32]),
    # Dict commands
    "tcl_dict_create": ("tcl", "dict_create", [], [ValType.I32]),
    "tcl_dict_get": ("tcl", "dict_get", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_dict_set": (
        "tcl",
        "dict_set",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_dict_exists": ("tcl", "dict_exists", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_dict_keys": ("tcl", "dict_keys", [ValType.I32], [ValType.I32]),
    "tcl_dict_values": ("tcl", "dict_values", [ValType.I32], [ValType.I32]),
    "tcl_dict_size": ("tcl", "dict_size", [ValType.I32], [ValType.I32]),
    "tcl_dict_merge_pair": (
        "tcl",
        "dict_merge_pair",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_error": ("tcl", "tcl_cmd_error", [ValType.I32], []),
    "tcl_format": (
        "tcl",
        "tcl_cmd_format",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_regexp": ("tcl", "tcl_cmd_regexp", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_open": ("tcl", "tcl_cmd_open", [ValType.I32], [ValType.I32]),
    "tcl_close": ("tcl", "tcl_cmd_close", [ValType.I32], [ValType.I32]),
    "tcl_read": ("tcl", "tcl_cmd_read", [ValType.I32], [ValType.I32]),
    "tcl_gets": ("tcl", "tcl_cmd_gets", [ValType.I32], [ValType.I32]),
    # Global variable table
    "tcl_global_set": ("tcl", "global_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_global_get": ("tcl", "global_get", [ValType.I32], [ValType.I32]),
    "tcl_global_exists": ("tcl", "global_exists", [ValType.I32], [ValType.I32]),
    # Catch / error handling
    "tcl_catch_enter": ("tcl", "catch_enter", [], []),
    "tcl_catch_leave": ("tcl", "catch_leave", [], [ValType.I32]),
    "tcl_catch_result": ("tcl", "catch_result", [], [ValType.I32]),
    "tcl_catch_has_error": ("tcl", "catch_has_error", [], [ValType.I32]),
    "tcl_catch_set_ok_result": ("tcl", "catch_set_ok_result", [ValType.I32], []),
    # Interpreter fallback
    "tcl_eval": ("tcl", "tcl_eval", [ValType.I32], [ValType.I32]),
    # Namespace context for eval-fallback calls — compiled procs
    # set the current namespace before ``tcl_eval`` so dynamic
    # ``proc $name`` / ``variable $name`` inside the fallback
    # qualify into the enclosing namespace instead of the global
    # scope.  ``ns_set`` returns an opaque i64 save token;
    # ``ns_restore`` unwinds it.
    "tcl_ns_set": (
        "tcl",
        "ns_set",
        [ValType.I32, ValType.I32],
        [ValType.I64],
    ),
    "tcl_ns_restore": ("tcl", "ns_restore", [ValType.I64], []),
    # Frame stack (local variable scoping)
    "tcl_frame_push": ("tcl", "frame_push", [], [ValType.I32]),
    "tcl_frame_pop": ("tcl", "frame_pop", [], []),
    # Per-frame invocation argv — set by the compiled-proc prologue
    # so ``info level 0`` / ``info level -N`` inside the body reads
    # the real invocation list rather than a placeholder.
    "tcl_frame_set_argv": ("tcl", "frame_set_argv", [ValType.I32], []),
    "tcl_frame_get_argv": ("tcl", "frame_get_argv", [ValType.I32], [ValType.I32]),
    # Pending ``argv0`` slot — a compiled caller writes the exact
    # word it invoked the callee with immediately before the
    # compiled ``call``; the callee's prologue reads-and-clears it
    # via ``take_pending_argv0`` so ``info level 0`` reports the
    # caller's word (including imported / renamed / qualified
    # forms) rather than the callee's registered qname tail.
    "tcl_frame_set_pending_argv0": (
        "tcl",
        "frame_set_pending_argv0",
        [ValType.I32],
        [],
    ),
    "tcl_frame_take_pending_argv0": (
        "tcl",
        "frame_take_pending_argv0",
        [],
        [ValType.I32],
    ),
    "tcl_var_resolve": ("tcl", "var_resolve", [ValType.I32], [ValType.I32]),
    "tcl_var_set": ("tcl", "var_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_var_exists": ("tcl", "var_exists", [ValType.I32], [ValType.I32]),
    "tcl_local_set": ("tcl", "local_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_local_get": ("tcl", "local_get", [ValType.I32], [ValType.I32]),
    # Proc registry
    "tcl_proc_register": (
        "tcl",
        "proc_register",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_proc_register_compiled": (
        "tcl",
        "proc_register_compiled",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Info command
    "tcl_info_exists": ("tcl", "info_exists", [ValType.I32], [ValType.I32]),
    "tcl_info_dispatch": ("tcl", "info_dispatch", [ValType.I32, ValType.I32], [ValType.I32]),
    # Clock command — WASI clock_time_get wrappers, integer results.
    "tcl_clock_seconds": ("tcl", "clock_seconds", [], [ValType.I32]),
    "tcl_clock_clicks": ("tcl", "clock_clicks", [], [ValType.I32]),
    "tcl_clock_milliseconds": ("tcl", "clock_milliseconds", [], [ValType.I32]),
    # Frame-depth helpers for uplevel — temporarily shift frame_depth
    # so a called ``tcl_eval`` runs at a caller's scope.
    "tcl_frame_depth_stash": (
        "tcl",
        "frame_depth_stash",
        [ValType.I32],
        [ValType.I32],
    ),
    "tcl_frame_depth_restore": ("tcl", "frame_depth_restore", [ValType.I32], []),
    # Arrays — dedicated per-array hash tables.
    "tcl_array_set": (
        "tcl",
        "array_set",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_get": ("tcl", "array_get", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_array_exists": ("tcl", "array_exists", [ValType.I32], [ValType.I32]),
    "tcl_array_element_exists": (
        "tcl",
        "array_element_exists",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_size": ("tcl", "array_size", [ValType.I32], [ValType.I32]),
    "tcl_array_unset": ("tcl", "array_unset", [ValType.I32], [ValType.I32]),
    "tcl_array_unset_element": (
        "tcl",
        "array_unset_element",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_names": (
        "tcl",
        "array_names",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # String split/join
    "tcl_split": ("tcl", "tcl_cmd_split", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_join": ("tcl", "tcl_cmd_join", [ValType.I32, ValType.I32], [ValType.I32]),
    # Diagnostic — record the current source site so trap paths can
    # prefix stderr with ``site=<id>`` for sidecar-map resolution.
    "tcl_diag_set": ("tcl", "diag_set", [ValType.I32], []),
    # I/O + channel stubs (tcl_io_stubs.zig) — trap with "unsupported
    # command: <name>".  Each stub takes i32 args and returns i32.
    "tcl_eof": ("tcl", "tcl_cmd_eof", [ValType.I32], [ValType.I32]),
    "tcl_flush": ("tcl", "tcl_cmd_flush", [ValType.I32], [ValType.I32]),
    "tcl_fblocked": ("tcl", "tcl_cmd_fblocked", [ValType.I32], [ValType.I32]),
    "tcl_fconfigure": ("tcl", "tcl_cmd_fconfigure", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_tell": ("tcl", "tcl_cmd_tell", [ValType.I32], [ValType.I32]),
    "tcl_seek": ("tcl", "tcl_cmd_seek", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_chan": ("tcl", "tcl_cmd_chan", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_fcopy": ("tcl", "tcl_cmd_fcopy", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_fileevent": ("tcl", "tcl_cmd_fileevent", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_socket": ("tcl", "tcl_cmd_socket", [ValType.I32, ValType.I32], [ValType.I32]),
    # Filesystem / process stubs (tcl_fs_stubs.zig).
    "tcl_file": (
        "tcl",
        "tcl_cmd_file",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_glob": ("tcl", "tcl_cmd_glob", [ValType.I32], [ValType.I32]),
    "tcl_pwd": ("tcl", "tcl_cmd_pwd", [], [ValType.I32]),
    "tcl_cd": ("tcl", "tcl_cmd_cd", [ValType.I32], [ValType.I32]),
    "tcl_exec": ("tcl", "tcl_cmd_exec", [ValType.I32], [ValType.I32]),
    "tcl_source": ("tcl", "tcl_cmd_source", [ValType.I32], [ValType.I32]),
    "tcl_load": ("tcl", "tcl_cmd_load", [ValType.I32], [ValType.I32]),
    "tcl_unload": ("tcl", "tcl_cmd_unload", [ValType.I32], [ValType.I32]),
    # Format / regex / encoding stubs (tcl_fmt_stubs.zig).
    "tcl_scan": ("tcl", "tcl_cmd_scan", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_binary": ("tcl", "tcl_cmd_binary", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_regsub": ("tcl", "tcl_cmd_regsub", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_encoding": (
        "tcl",
        "tcl_cmd_encoding",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Time / event stubs (tcl_time_stubs.zig).  clock_format /
    # clock_scan / clock_add are the non-implemented clock
    # subcommands; the arithmetic ones (seconds / clicks /
    # milliseconds) are real runtime fns in tcl_clock.zig.
    "tcl_clock_format": ("tcl", "clock_format", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_clock_scan": ("tcl", "clock_scan", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_clock_add": ("tcl", "clock_add", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_after": ("tcl", "tcl_cmd_after", [ValType.I32], [ValType.I32]),
    "tcl_vwait": ("tcl", "tcl_cmd_vwait", [ValType.I32], [ValType.I32]),
    "tcl_update": ("tcl", "tcl_cmd_update", [], [ValType.I32]),
    "tcl_coroutine": ("tcl", "tcl_cmd_coroutine", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_yield": ("tcl", "tcl_cmd_yield", [ValType.I32], [ValType.I32]),
    "tcl_yieldto": ("tcl", "tcl_cmd_yieldto", [ValType.I32], [ValType.I32]),
    # Environment / metadata stubs (tcl_env_stubs.zig).  ``namespace``
    # and ``namespace eval`` are handled separately by the compiler;
    # this stub catches the other namespace subcommands falling
    # through.
    "tcl_namespace": ("tcl", "namespace", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_package": ("tcl", "tcl_cmd_package_cmd", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_trace": ("tcl", "tcl_cmd_trace_cmd", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_interp": ("tcl", "tcl_cmd_interp_cmd", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_apply": ("tcl", "tcl_cmd_apply", [ValType.I32, ValType.I32], [ValType.I32]),
}

# Import keys for the TclObj lifecycle functions — always registered
_OBJ_LIFECYCLE_IMPORTS = frozenset(
    {
        "tcl_obj_new_int",
        "tcl_obj_new_string",
        "tcl_obj_get_int",
    }
)

# Commands that map to runtime imports.  The value is
# (import_key, arg_count_or_None) where ``None`` means variadic.
#
# Stub-mapped commands (file, glob, encoding, trace, etc.) route
# through this dispatch table so the compiler calls the stub
# directly — producing a clean ``unsupported command: <name>`` trap
# — instead of generating a ``tcl_eval`` fallback that invokes the
# interpreter which then fails to find the command.
_CMD_RUNTIME: dict[str, tuple[str, int | None]] = {
    # Implemented runtime functions
    "puts": ("tcl_puts", 1),
    "append": ("tcl_append", 2),
    "llength": ("tcl_list_length", 1),
    "lappend": ("tcl_lappend", 2),
    "lindex": ("tcl_list_index", 2),
    "lrange": ("tcl_list_range", 3),
    "lsort": ("tcl_list_sort", 1),
    "lreverse": ("tcl_list_reverse", 1),
    "lrepeat": ("tcl_list_repeat", 2),
    "linsert": ("tcl_list_insert", 3),
    "lreplace": ("tcl_list_replace", 4),
    # ``lset`` is intentionally NOT here.  Placing it in
    # ``_CMD_RUNTIME`` would also register a generic value-context
    # dispatch path (``_emit_command_expr``) that passes the varname
    # as a literal list operand — which produces the wrong result for
    # ``set ret [lset lst 1 X]`` (lst is a var, not a literal list).
    # ``lset`` has a dedicated statement-context emitter
    # (``_emit_cmd_lset``); the import ``tcl_list_set`` is registered
    # explicitly by the scan phase (see ``_scan.py``) whenever an
    # ``IRCall(command="lset")`` appears.  Value-context
    # ``[lset …]`` falls through to the generic eval fallback.
    "lsearch": ("tcl_list_search", 2),
    "concat": ("tcl_concat", 2),
    "error": ("tcl_error", 1),
    "split": ("tcl_split", 2),
    "join": ("tcl_join", 2),
    # I/O stubs — open/close/read/gets/eof/flush/fblocked/fconfigure/
    # tell/seek/chan/fcopy/fileevent/socket all trap with
    # "unsupported command: <name>".
    "open": ("tcl_open", 1),
    "close": ("tcl_close", 1),
    "read": ("tcl_read", 1),
    "gets": ("tcl_gets", 1),
    "eof": ("tcl_eof", 1),
    "flush": ("tcl_flush", 1),
    "fblocked": ("tcl_fblocked", 1),
    "fconfigure": ("tcl_fconfigure", 2),
    "tell": ("tcl_tell", 1),
    "seek": ("tcl_seek", 2),
    "chan": ("tcl_chan", 2),
    "fcopy": ("tcl_fcopy", 2),
    "fileevent": ("tcl_fileevent", 2),
    "socket": ("tcl_socket", 2),
    # Filesystem / process stubs.
    "file": ("tcl_file", 3),
    "glob": ("tcl_glob", 1),
    "pwd": ("tcl_pwd", 0),
    "cd": ("tcl_cd", 1),
    "exec": ("tcl_exec", 1),
    "source": ("tcl_source", 1),
    "load": ("tcl_load", 1),
    "unload": ("tcl_unload", 1),
    # Format / regex / encoding stubs.
    "format": ("tcl_format", 4),
    # ``scan`` and ``binary`` are no longer in _CMD_RUNTIME — they route through
    # the eval fallback so the interpreter can see all args (varnames for scan,
    # format-values for binary format, subSpec/varName for regsub).  The
    # tcl_cmd_scan / tcl_cmd_binary / tcl_cmd_regsub exports remain in the
    # runtime for ABI continuity with old compiled modules.
    "regexp": ("tcl_regexp", 2),
    "encoding": ("tcl_encoding", 3),
    # Event / coroutine stubs.  The arithmetic clock commands
    # (``clock seconds`` / ``clock clicks`` / ``clock milliseconds``)
    # are real runtime fns and route through a separate dispatcher
    # in ``_emit_cmd_clock``; only the formatting variants surface
    # here.
    "after": ("tcl_after", 1),
    "vwait": ("tcl_vwait", 1),
    "update": ("tcl_update", 0),
    "coroutine": ("tcl_coroutine", 2),
    "yield": ("tcl_yield", 1),
    "yieldto": ("tcl_yieldto", 1),
    # Environment / metadata stubs.  ``namespace eval`` is compiled
    # (it is a control-flow construct); the ``namespace`` entry here
    # only catches the other subcommands (current, qualifiers, which,
    # tail, code, delete, import, export, exists, parent, children,
    # inscope, origin, forget, path, ensemble) that route through
    # this dispatch.  ``interp`` used to live here too, pointing at
    # the trapping ``tcl_cmd_interp_cmd`` stub — since the runtime
    # added real ``interp alias`` + ``interp hide`` / ``interp
    # expose`` / ``interp hidden`` support (see
    # docs/design/runtime/rename-alias.md and
    # docs/design/runtime/command-introspection.md) the codegen
    # now routes ``interp`` through the eval fallback so the
    # interpreter's ``interp`` built-in handles them.  Child-interp
    # subcommands (``interp create`` / ``slaves`` / ``eval``) still
    # trap cleanly via ``tcl_env_stubs``.
    "package": ("tcl_package", 2),
    "trace": ("tcl_trace", 2),
    "apply": ("tcl_apply", 2),
}

# Runtime commands whose Zig implementation is total for well-typed
# inputs — they return a result for every argument shape the compiler
# can emit and never trap into ``tcl_diag``.  The codegen elides the
# per-call ``tcl_diag_set`` preamble (~4 WASM bytes + one DiagSite
# record) for these commands because the trap-resolver
# (:func:`tests.test_wasm_real_tcl._resolve_trap`) only reads diag
# sites when a ``tcl trap: site=<id>`` line appears on stderr, and
# none of the commands below emit that.
#
# Commands omitted from this set (i.e. that DO need a diag site):
# every I/O / FS / event / coroutine / introspection stub that can
# raise "unsupported command: X"; ``format``/``scan``/``regexp`` (may
# error on bad patterns); ``error`` itself; ``lsort``/``lsearch`` /
# ``split``/``join`` (may error on malformed lists); ``string is *``;
# all ``tcl_dict_*`` (dict-shape errors); all ``tcl_clock_*`` (stub
# error paths).
_CMD_RUNTIME_NONTRAPPING: frozenset[str] = frozenset(
    {
        # ``puts`` writes to a WASI stdout pipe the host always
        # provides; there is no "channel closed" error path the
        # codegen can reach.
        "puts",
        # ``append`` concatenates strings verbatim — no list parsing,
        # no number conversion, no shimmer.
        "append",
    }
)

# String sub-command → import key
_STRING_SUBCMD_IMPORT: dict[str, str] = {
    "length": "tcl_string_length",
    "index": "tcl_string_index",
    "range": "tcl_string_range",
    "compare": "tcl_string_compare",
    "match": "tcl_string_match",
    "map": "tcl_string_map",
    "trim": "tcl_string_trim",
    "trimleft": "tcl_string_trimleft",
    "trimright": "tcl_string_trimright",
    "equal": "tcl_string_equal",
    "first": "tcl_string_first",
    "last": "tcl_string_last",
    "repeat": "tcl_string_repeat",
    "reverse": "tcl_string_reverse",
    "toupper": "tcl_string_toupper",
    "tolower": "tcl_string_tolower",
    "totitle": "tcl_string_totitle",
    "replace": "tcl_string_replace",
}

# ``string is <class> <value>`` sub-sub-command → import key
_STRING_IS_IMPORT: dict[str, str] = {
    "integer": "tcl_string_is_integer",
    "alpha": "tcl_string_is_alpha",
    "digit": "tcl_string_is_digit",
    "space": "tcl_string_is_space",
}

# Dict sub-command → (import key, additional_arg_count after dict_var)
# ``create`` and ``merge`` are intentionally NOT in this map — the
# compiler specialises them in ``_emit_cmd_dict`` (and the
# value-context equivalent) to fold/chain at compile time with
# ``tcl_lappend`` / ``tcl_dict_merge_pair`` respectively, bypassing
# the generic dispatch below.
_DICT_SUBCMD_IMPORT: dict[str, str] = {
    "get": "tcl_dict_get",
    "set": "tcl_dict_set",
    "exists": "tcl_dict_exists",
    "keys": "tcl_dict_keys",
    "values": "tcl_dict_values",
    "size": "tcl_dict_size",
}

# ``clock <subcmd>`` → import key.  Only subcommands that map to a
# WASI-backed runtime hook are listed; ``format``/``scan`` fall through
# to the interpreter which itself traps in the sandbox.
_CLOCK_SUBCMD_IMPORT: dict[str, str] = {
    "seconds": "tcl_clock_seconds",
    "clicks": "tcl_clock_clicks",
    "milliseconds": "tcl_clock_milliseconds",
}

# Commands that are scope declarations, compile-time-only, or CFG
# placeholders — NOPs in WASM.
#
# ``rename`` is explicitly NOT listed here even though it was treated
# as a NOP by the pre-runtime-rename-alias wave — the runtime now
# implements real rename semantics (``tcl_rename.rename_command`` via
# the interpreter's ``rename`` built-in), so the codegen must route
# every ``rename`` call through the eval fallback rather than dropping
# it.  See docs/design/runtime/rename-alias.md.
_SCOPE_NOP_COMMANDS = frozenset(
    {
        "namespace",
        "proc",
        "package",
        # CFG placeholders — the actual logic is emitted by the loop/foreach
        # emitters; these IRCall nodes are just iteration-setup markers.
        "foreach",
    }
)

# Commands that require capabilities unavailable in the WASM sandbox.
# The codegen emits a call to the runtime ``error`` function with a
# descriptive message so the module traps with a clear diagnostic
# rather than silently emitting a NOP.
# Commands that have no meaningful implementation in WASM and
# aren't covered by the ``_CMD_RUNTIME`` stub table.  Kept as an
# explicit trap path so (a) we reject them at compile time with a
# clear message and (b) new users see they're intentionally
# unimplemented rather than silently accepted.  Most commands that
# used to live here (``exec``, ``socket``, ``interp``, ``after``,
# etc.) have moved to the stub dispatch table — they still trap,
# but the diag machinery attributes the trap to a source site.
_UNSUPPORTED_COMMANDS: frozenset[str] = frozenset()
