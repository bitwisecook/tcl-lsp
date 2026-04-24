"""Runtime import tables consumed by the WASM codegen.

Each table maps a compile-time key to the Zig-exported runtime
function (or a structured descriptor) the emitter calls into.
Separated from the emitter so import tables can be audited,
extended, and stubbed independently of codegen changes.
"""

from __future__ import annotations

from ....commands.registry import REGISTRY, WasmRuntimeImport
from ._ir import ValType


def runtime_import_for(command: str) -> WasmRuntimeImport | None:
    """Return the runtime-import descriptor for *command*, or ``None``.

    Consults every ``CommandSpec`` registered under *command* and
    returns the first ``wasm_runtime_import`` declaration found.  The
    single source of truth for "does this Tcl command dispatch to a
    Zig runtime import, and what import key?"  Used by the scan phase
    (to register the import in the WASM imports section) and by the
    emitter (to pick the function index and decide whether a diag site
    is needed).
    """
    specs = REGISTRY.specs_by_name.get(command)
    if specs is None:
        return None
    for spec in specs:
        if spec.wasm_runtime_import is not None:
            return spec.wasm_runtime_import
    return None


def subcommand_runtime_import_for(
    command: str, subcommand: str
) -> WasmRuntimeImport | None:
    """Return the runtime-import descriptor for ``<command> <subcommand>``.

    Replaces the ``_STRING_SUBCMD_IMPORT`` / ``_DICT_SUBCMD_IMPORT`` /
    ``_CLOCK_SUBCMD_IMPORT`` shadow tables.  Looks up the
    ``SubCommand`` entry on any ``CommandSpec`` for *command* and
    returns its ``wasm_runtime_import`` if set.
    """
    specs = REGISTRY.specs_by_name.get(command)
    if specs is None:
        return None
    for spec in specs:
        sub = spec.subcommands.get(subcommand)
        if sub is not None and sub.wasm_runtime_import is not None:
            return sub.wasm_runtime_import
    return None

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
    "tcl_obj_new_float": ("tcl", "obj_new_float", [ValType.F64], [ValType.I32]),
    "tcl_obj_get_float": ("tcl", "obj_get_float", [ValType.I32], [ValType.F64]),
    # Float-aware arithmetic — handle int/float dispatch at runtime
    "tcl_arith_add": ("tcl", "tcl_arith_add", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_sub": ("tcl", "tcl_arith_sub", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_mul": ("tcl", "tcl_arith_mul", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_div": ("tcl", "tcl_arith_div", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_mod": ("tcl", "tcl_arith_mod", [ValType.I32, ValType.I32], [ValType.I32]),
    # Math functions
    "tcl_math_double": ("tcl", "tcl_math_double", [ValType.I32], [ValType.I32]),
    "tcl_math_int": ("tcl", "tcl_math_int", [ValType.I32], [ValType.I32]),
    "tcl_math_round": ("tcl", "tcl_math_round", [ValType.I32], [ValType.I32]),
    "tcl_math_log": ("tcl", "tcl_math_log", [ValType.I32], [ValType.I32]),
    "tcl_math_sqrt": ("tcl", "tcl_math_sqrt", [ValType.I32], [ValType.I32]),
    "tcl_math_exp": ("tcl", "tcl_math_exp", [ValType.I32], [ValType.I32]),
    "tcl_math_log10": ("tcl", "tcl_math_log10", [ValType.I32], [ValType.I32]),
    "tcl_math_sin": ("tcl", "tcl_math_sin", [ValType.I32], [ValType.I32]),
    "tcl_math_cos": ("tcl", "tcl_math_cos", [ValType.I32], [ValType.I32]),
    "tcl_math_fabs": ("tcl", "tcl_math_fabs", [ValType.I32], [ValType.I32]),
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
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32, ValType.I32],
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

# ``string is <class> <value>`` sub-sub-command → import key.
# Unlike the ordinary ``<command> <subcommand>`` dispatch (which lives
# on ``SubCommand.wasm_runtime_import``), this table dispatches on the
# *second* argument of ``string is`` — i.e. the character-class name.
# The registry doesn't model sub-sub-commands, so it stays as a dict.
_STRING_IS_IMPORT: dict[str, str] = {
    "integer": "tcl_string_is_integer",
    "alpha": "tcl_string_is_alpha",
    "digit": "tcl_string_is_digit",
    "space": "tcl_string_is_space",
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
