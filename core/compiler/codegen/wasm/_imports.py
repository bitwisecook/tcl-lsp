"""Runtime import tables consumed by the WASM codegen.

Each table maps a compile-time key to the Zig-exported runtime
function (or a structured descriptor) the emitter calls into.
Separated from the emitter so import tables can be audited,
extended, and stubbed independently of codegen changes.
"""

from __future__ import annotations

from ....commands.registry import REGISTRY, WasmRuntimeImport
from ._ir import ValType


def _str_to_valtype(s: str) -> ValType:
    """Convert a lowercase type name from a spec (``"i32"``) to a ValType."""
    return getattr(ValType, s.upper())


def import_signature(
    key: str,
) -> tuple[str, str, list[ValType], list[ValType]] | None:
    """Return ``(module, export_name, params, results)`` for an import key.

    Prefers the spec-side ``WasmRuntimeImport`` (the registry is the
    source of truth for command-owned imports), falling back to
    :data:`_INFRASTRUCTURE_IMPORTS` for helpers that don't have a
    command owner (obj lifecycle, arith, math funcs, frame ops,
    diag, ``tcl_eval``, etc.).  Returns ``None`` when the key isn't
    known on either side.
    """
    spec_entry = _spec_import_table().get(key)
    if spec_entry is not None:
        rimp = spec_entry
        if rimp.params or rimp.results:
            return (
                rimp.module,
                rimp.resolved_export_name,
                [_str_to_valtype(p) for p in rimp.params],
                [_str_to_valtype(r) for r in rimp.results],
            )
    infra = _INFRASTRUCTURE_IMPORTS.get(key)
    if infra is not None:
        return infra
    return None


_SPEC_IMPORT_TABLE: dict[str, WasmRuntimeImport] | None = None


def _spec_import_table() -> dict[str, WasmRuntimeImport]:
    """Lazy-build ``{import_key: WasmRuntimeImport}`` by scanning the registry.

    Computed once on first access.  If new specs are loaded later the
    table should be invalidated, but in practice the WASM codegen
    loads all specs before any import is queried.
    """
    global _SPEC_IMPORT_TABLE
    if _SPEC_IMPORT_TABLE is not None:
        return _SPEC_IMPORT_TABLE
    table: dict[str, WasmRuntimeImport] = {}
    for specs in REGISTRY.specs_by_name.values():
        for spec in specs:
            rimp = spec.wasm_runtime_import
            if rimp is not None:
                table.setdefault(rimp.import_key, rimp)
            for sub in spec.subcommands.values():
                srimp = sub.wasm_runtime_import
                if srimp is not None:
                    table.setdefault(srimp.import_key, srimp)
    _SPEC_IMPORT_TABLE = table
    return table


def runtime_import_for(command: str) -> WasmRuntimeImport | None:
    """Return the runtime-import descriptor for *command*, or ``None``.

    Consults every ``CommandSpec`` registered under *command* and
    returns the first ``wasm_runtime_import`` declaration found.  The
    single source of truth for "does this Tcl command dispatch to a
    Zig runtime import, and what import key?"  Used by the scan phase
    (to register the import in the WASM imports section) and by the
    emitter (to pick the function index and decide whether a diag site
    is needed).

    Accepts both bare (``set``) and explicitly-qualified (``::set``)
    forms; the qualified form is the canonical spelling stamped on
    ``IRCall.canonical_command`` by lowering.  See issue #246.
    """
    specs = REGISTRY.specs_by_name.get(command)
    if specs is None and command.startswith("::"):
        specs = REGISTRY.specs_by_name.get(command[2:])
    if specs is None:
        return None
    for spec in specs:
        if spec.wasm_runtime_import is not None:
            return spec.wasm_runtime_import
    return None


def command_emits_nothing(command: str) -> bool:
    """Return ``True`` if *command* produces no value on the operand stack.

    Consults ``CommandSpec.wasm_emits_nothing`` on every registered
    spec for *command*.  Replaces the ``_SCOPE_NOP_COMMANDS`` shadow
    frozenset — the registry is the sole source of truth.

    Accepts both bare (``set``) and qualified (``::set``) forms.
    """
    specs = REGISTRY.specs_by_name.get(command)
    if specs is None and command.startswith("::"):
        specs = REGISTRY.specs_by_name.get(command[2:])
    if specs is None:
        return False
    return any(spec.wasm_emits_nothing for spec in specs)


def subcommand_runtime_import_for(command: str, subcommand: str) -> WasmRuntimeImport | None:
    """Return the runtime-import descriptor for ``<command> <subcommand>``.

    Replaces the ``_STRING_SUBCMD_IMPORT`` / ``_DICT_SUBCMD_IMPORT`` /
    ``_CLOCK_SUBCMD_IMPORT`` shadow tables.  Looks up the
    ``SubCommand`` entry on any ``CommandSpec`` for *command* and
    returns its ``wasm_runtime_import`` if set.

    Accepts both bare (``string``) and qualified (``::string``) forms.
    """
    specs = REGISTRY.specs_by_name.get(command)
    if specs is None and command.startswith("::"):
        specs = REGISTRY.specs_by_name.get(command[2:])
    if specs is None:
        return None
    for spec in specs:
        sub = spec.subcommands.get(subcommand)
        if sub is not None and sub.wasm_runtime_import is not None:
            return sub.wasm_runtime_import
    return None


def subcommand_min_arity_for(command: str, subcommand: str) -> int | None:
    """Return ``SubCommand.arity.min`` for ``<command> <subcommand>``, or None.

    Used by the static-call codegen to bail out to ``eval``-fallback when
    a literal call is short of the registry-declared minimum.  Without
    this guard, e.g. statement-level ``catch {string index} b`` zero-pads
    the missing args at compile time, ``tcl_string_index(0,0)`` silently
    returns the empty string, and the surrounding ``catch`` never sees an
    error — ``$b`` ends up empty and ``::errorInfo`` is never stamped
    (error-1.3 / 1.6 / 1.7 / cmdAH-1.4 / 1.5 / list).
    """
    specs = REGISTRY.specs_by_name.get(command)
    if specs is None and command.startswith("::"):
        specs = REGISTRY.specs_by_name.get(command[2:])
    if specs is None:
        return None
    for spec in specs:
        sub = spec.subcommands.get(subcommand)
        if sub is not None:
            return sub.arity.min
    return None


# Infrastructure import signatures — runtime helpers the codegen
# references directly, with no ``CommandSpec`` owner.  Every entry
# maps an import key to ``(module, export_name, param_types,
# result_types)``.  Command-owned imports (``tcl_puts``,
# ``tcl_lappend``, every ``string <sub>``, every ``dict <sub>``, etc.)
# live on :class:`WasmRuntimeImport` fields on their specs instead —
# :func:`import_signature` consults the registry first and falls back
# to this dict only for infrastructure keys.
#
# Values are represented as i32 TclObj pointers.  The TclObj lifecycle
# imports bridge between raw WASM integers (i64) used in expr arithmetic
# and the i32 object pointers passed to/from runtime functions and
# stored in locals.
_INFRASTRUCTURE_IMPORTS: dict[str, tuple[str, str, list[ValType], list[ValType]]] = {
    # TclObj lifecycle — bridges raw WASM ints (i64) to i32 TclObj pointers.
    "tcl_obj_new_int": ("tcl", "obj_new_int", [ValType.I64], [ValType.I32]),
    "tcl_obj_new_string": ("tcl", "obj_new_string", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_obj_get_int": ("tcl", "obj_get_int", [ValType.I32], [ValType.I64]),
    "tcl_obj_new_float": ("tcl", "obj_new_float", [ValType.F64], [ValType.I32]),
    "tcl_obj_get_float": ("tcl", "obj_get_float", [ValType.I32], [ValType.F64]),
    # Refcount discipline (S2): ``retain`` is null-safe (matches
    # ``release``) so the codegen can wrap every owned-slot write
    # unconditionally and release each owned slot in the epilogue
    # without per-slot null checks.  Only emitted by frame-elided
    # procs from S2.2 onwards.
    "tcl_obj_retain": ("tcl", "tcl_obj_retain", [ValType.I32], []),
    "tcl_obj_release": ("tcl", "tcl_obj_release", [ValType.I32], []),
    # Float-aware arithmetic — int/float dispatch happens runtime-side.
    "tcl_arith_add": ("tcl", "tcl_arith_add", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_sub": ("tcl", "tcl_arith_sub", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_mul": ("tcl", "tcl_arith_mul", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_div": ("tcl", "tcl_arith_div", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_mod": ("tcl", "tcl_arith_mod", [ValType.I32, ValType.I32], [ValType.I32]),
    # Bitwise / shift — strict integer domain.  Float operands raise
    # ``cannot use floating-point value …``; negative shift counts
    # raise ``negative shift argument``.  See issues #260, #261.
    "tcl_arith_lshift": ("tcl", "tcl_arith_lshift", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_rshift": ("tcl", "tcl_arith_rshift", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_band": ("tcl", "tcl_arith_band", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_bor": ("tcl", "tcl_arith_bor", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_bxor": ("tcl", "tcl_arith_bxor", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_arith_bnot": ("tcl", "tcl_arith_bnot", [ValType.I32], [ValType.I32]),
    # Float-preserving unary negation — keeps TYPE_FLOAT tag through
    # ``-$x`` so the bitwise / shift domain checks downstream observe
    # the float and raise the canonical error (Codex review on
    # PR #287; issue #261).
    "tcl_arith_neg": ("tcl", "tcl_arith_neg", [ValType.I32], [ValType.I32]),
    # Unary ``+``: identity on numeric operands, raises ``cannot use
    # non-numeric string …`` on string operands.  Without this the
    # inline no-op path silently passed through ``+"a"`` (expr-old
    # 5.2).
    "tcl_arith_pos": ("tcl", "tcl_arith_pos", [ValType.I32], [ValType.I32]),
    # ``**`` operator — bignum-aware integer exponentiation.  The
    # WASM emitter previously inlined an i64 multiplication loop
    # (``_emit_power``) which silently wrapped past the i64 boundary;
    # this helper promotes overflowing results to TYPE_BIGNUM via
    # ``Managed.pow``, matching upstream Tcl 9.0 semantics.
    "tcl_arith_pow": ("tcl", "tcl_arith_pow", [ValType.I32, ValType.I32], [ValType.I32]),
    # Strict ``incr`` helper — validates the variable's value as a
    # strict integer (rejects float strings like ``"52.60"`` and
    # boolean keywords) before adding the amount.  Issue #262.
    "tcl_incr": ("tcl", "tcl_incr", [ValType.I32, ValType.I32], [ValType.I32]),
    # Math functions — called from expr codegen by name.
    "tcl_math_double": ("tcl", "tcl_math_double", [ValType.I32], [ValType.I32]),
    "tcl_math_int": ("tcl", "tcl_math_int", [ValType.I32], [ValType.I32]),
    # ``wide(x)`` — signed 64-bit truncation of ``int(x)``.  Distinct
    # from ``tcl_math_int`` (which preserves bignum precision) because
    # reference Tcl's ``ExprWideFunc`` calls ``ExprIntFunc`` then
    # ``TclGetWideBitsFromObj`` to extract the low 64 bits — used by
    # expr-old-34.13 / 34.14 (``wide(1.0e30)`` truncates to
    # ``5076964154930102272``).
    "tcl_math_wide": ("tcl", "tcl_math_wide", [ValType.I32], [ValType.I32]),
    "tcl_math_round": ("tcl", "tcl_math_round", [ValType.I32], [ValType.I32]),
    "tcl_math_log": ("tcl", "tcl_math_log", [ValType.I32], [ValType.I32]),
    "tcl_math_sqrt": ("tcl", "tcl_math_sqrt", [ValType.I32], [ValType.I32]),
    "tcl_math_exp": ("tcl", "tcl_math_exp", [ValType.I32], [ValType.I32]),
    "tcl_math_log10": ("tcl", "tcl_math_log10", [ValType.I32], [ValType.I32]),
    "tcl_math_sin": ("tcl", "tcl_math_sin", [ValType.I32], [ValType.I32]),
    "tcl_math_cos": ("tcl", "tcl_math_cos", [ValType.I32], [ValType.I32]),
    "tcl_math_tan": ("tcl", "tcl_math_tan", [ValType.I32], [ValType.I32]),
    "tcl_math_asin": ("tcl", "tcl_math_asin", [ValType.I32], [ValType.I32]),
    "tcl_math_acos": ("tcl", "tcl_math_acos", [ValType.I32], [ValType.I32]),
    "tcl_math_atan": ("tcl", "tcl_math_atan", [ValType.I32], [ValType.I32]),
    "tcl_math_atan2": ("tcl", "tcl_math_atan2", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_math_sinh": ("tcl", "tcl_math_sinh", [ValType.I32], [ValType.I32]),
    "tcl_math_cosh": ("tcl", "tcl_math_cosh", [ValType.I32], [ValType.I32]),
    "tcl_math_tanh": ("tcl", "tcl_math_tanh", [ValType.I32], [ValType.I32]),
    "tcl_math_floor": ("tcl", "tcl_math_floor", [ValType.I32], [ValType.I32]),
    "tcl_math_ceil": ("tcl", "tcl_math_ceil", [ValType.I32], [ValType.I32]),
    "tcl_math_fmod": ("tcl", "tcl_math_fmod", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_math_hypot": ("tcl", "tcl_math_hypot", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_math_fabs": ("tcl", "tcl_math_fabs", [ValType.I32], [ValType.I32]),
    # ``isqrt(x)`` — bignum-aware integer square root (expr-47.x).
    "tcl_math_isqrt": ("tcl", "tcl_math_isqrt", [ValType.I32], [ValType.I32]),
    # ``bool(x)`` — coerce to Tcl boolean accepting keyword forms
    # (``yes`` / ``no`` / ``true`` / ``false`` / ``on`` / ``off``).
    "tcl_math_bool": ("tcl", "tcl_math_bool", [ValType.I32], [ValType.I32]),
    # IEEE-754 float classification (TIP 519).  Each predicate takes
    # one operand and returns 0 / 1; ``fpclassify`` returns the
    # classification name as a string (``zero``/``subnormal``/...);
    # ``isunordered`` takes two operands.
    "tcl_math_isfinite": ("tcl", "tcl_math_isfinite", [ValType.I32], [ValType.I32]),
    "tcl_math_isinf": ("tcl", "tcl_math_isinf", [ValType.I32], [ValType.I32]),
    "tcl_math_isnan": ("tcl", "tcl_math_isnan", [ValType.I32], [ValType.I32]),
    "tcl_math_isnormal": ("tcl", "tcl_math_isnormal", [ValType.I32], [ValType.I32]),
    "tcl_math_issubnormal": (
        "tcl",
        "tcl_math_issubnormal",
        [ValType.I32],
        [ValType.I32],
    ),
    "tcl_math_fpclassify": (
        "tcl",
        "tcl_math_fpclassify",
        [ValType.I32],
        [ValType.I32],
    ),
    "tcl_math_isunordered": (
        "tcl",
        "tcl_math_isunordered",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # ``tcl_expr_unordered(a, b)`` — returns 1 if either operand is a
    # NaN value (TYPE_FLOAT NaN or ``"NaN"`` string).  Used by the
    # comparison codegen to short-circuit IEEE-754 unordered results.
    "tcl_expr_unordered": (
        "tcl",
        "tcl_expr_unordered",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # ``tcl_expr_eq_nan_aware(a, b)`` — returns 1 if *a* and *b* are
    # equal under Tcl 9 ``==`` semantics, 0 otherwise.  Differs from
    # ``tcl_expr_order_cmp`` by returning 0 when either operand is a
    # NaN value (IEEE-754: NaN is unordered with respect to
    # everything, including itself).  Used by the ``==`` / ``!=``
    # codegen so ``expr {NaN == NaN}`` returns 0 instead of the
    # bytewise-equal 1.
    "tcl_expr_eq_nan_aware": (
        "tcl",
        "tcl_expr_eq_nan_aware",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Canonicalise an obj via the expression parser — single-token
    # ``expr {$var}`` semantics.  Returns the canonical numeric obj
    # when the string parses as a Tcl integer / hex / octal / binary /
    # float; otherwise the original obj.
    "tcl_expr_canonicalise": (
        "tcl",
        "tcl_expr_canonicalise",
        [ValType.I32],
        [ValType.I32],
    ),
    # Infra used by the puts specialisation for ``-nonewline``.
    "tcl_puts_nonewline": (
        "tcl",
        "tcl_cmd_puts_nonewline",
        [ValType.I32],
        [ValType.I32],
    ),
    # Channel-form puts — used by the ``puts $chan msg`` and
    # ``puts -nonewline $chan msg`` shapes.  Routes the write through
    # the channel registry in ``runtime/zig/io/tcl_chan.zig`` so the
    # message lands on the right fd.  The third arg is a flag (0 =
    # append newline, 1 = ``-nonewline``).  Emitting this directly
    # avoids the eval-fallback path, which would round-trip the
    # quoted-string message through list-quote and silently brace it
    # (suppressing ``$var`` / ``[cmd]`` substitution).
    "tcl_puts_chan": (
        "tcl",
        "tcl_cmd_puts_chan",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # ``tcl_expr_order_cmp`` is used by expr codegen to compare TclObjs.
    "tcl_expr_order_cmp": (
        "tcl",
        "tcl_expr_order_cmp",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # ``string is <class>`` sub-sub-command — see :data:`_STRING_IS_IMPORT`
    # for the dispatch table; these signatures live here because the
    # registry doesn't model sub-sub-commands.
    "tcl_string_is_integer": ("tcl", "string_is_integer", [ValType.I32], [ValType.I32]),
    "tcl_string_is_wideinteger": ("tcl", "string_is_wideinteger", [ValType.I32], [ValType.I32]),
    "tcl_string_is_alpha": ("tcl", "string_is_alpha", [ValType.I32], [ValType.I32]),
    "tcl_string_is_digit": ("tcl", "string_is_digit", [ValType.I32], [ValType.I32]),
    "tcl_string_is_space": ("tcl", "string_is_space", [ValType.I32], [ValType.I32]),
    # List construction / helpers called by codegen paths that don't
    # route through a ``CommandSpec`` — e.g. ``_emit_list_value``
    # builds N-ary lists via ``tcl_list`` pair-chaining.
    "tcl_list_create": ("tcl", "tcl_list", [ValType.I32, ValType.I32], [ValType.I32]),
    # Multi-value ``lreplace`` dispatch — takes the values to insert
    # as a pre-built Tcl LIST so the runtime can resolve ``end-N``
    # against the original list ONCE and splice each value at the
    # right absolute slot, instead of having codegen chain
    # ``tcl_list_insert`` calls that re-resolve against the shrinking
    # list (lreplace-4.7.1 / lreplace-4.11).
    "tcl_cmd_lreplace_list": (
        "tcl",
        "tcl_cmd_lreplace_list",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Variadic ``format`` — ``tcl_cmd_format`` only accepts 3 subst
    # args; format callsites with more (or with ``%-*s`` / ``%.*s``
    # consuming two args per spec) route through the list-form
    # helper.  See ``codegen/wasm/_emitter/cmds/format_.py``.
    "tcl_cmd_format_list": (
        "tcl",
        "tcl_cmd_format_list",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_tail": ("tcl", "list_tail", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_list_contains": (
        "tcl",
        "tcl_cmd_list_contains",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_list_set": (
        "tcl",
        "tcl_cmd_list_set",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Dict helpers for the ``create`` / ``merge`` specialisations.
    "tcl_dict_create": ("tcl", "dict_create", [], [ValType.I32]),
    "tcl_dict_merge_pair": (
        "tcl",
        "dict_merge_pair",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Global variable table.
    "tcl_global_set": ("tcl", "global_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_global_get": ("tcl", "global_get", [ValType.I32], [ValType.I32]),
    # Strict variant of ``global_get`` — raises ``can't read "<name>":
    # no such variable`` when the global has never been set.  Used by
    # the codegen for ``$var`` substitutions / ``set var`` reads /
    # ``expr {$var}`` operands so the WASM backend matches the Python
    # VM and reference Tcl on missing-variable errors.  The lenient
    # ``tcl_global_get`` is still used by ``info exists`` / ``unset
    # -nocomplain`` / frame readback / the ``global`` command's pre-load.
    "tcl_global_get_or_error": ("tcl", "global_get_or_error", [ValType.I32], [ValType.I32]),
    "tcl_global_exists": ("tcl", "global_exists", [ValType.I32], [ValType.I32]),
    # Helper for the WASM-local-mirror read path — when a compiled
    # proc-local was never assigned (slot stays at 0) we need to raise
    # the standard Tcl missing-variable error.  Takes the name TclObj,
    # builds ``can't read "<name>": no such variable`` and routes
    # through the standard error path (catch / trap).
    "tcl_var_unset_error": ("tcl", "var_unset_error", [ValType.I32], []),
    # Catch / error handling.
    "tcl_catch_enter": ("tcl", "catch_enter", [], []),
    "tcl_catch_leave": ("tcl", "catch_leave", [], [ValType.I32]),
    "tcl_catch_result": ("tcl", "catch_result", [], [ValType.I32]),
    "tcl_catch_has_error": ("tcl", "catch_has_error", [], [ValType.I32]),
    "tcl_catch_set_ok_result": ("tcl", "catch_set_ok_result", [ValType.I32], []),
    # ``return ?value?`` from inside a compile-time ``catch`` body —
    # sets ``return_flag`` + ``return_val`` in the runtime.  The codegen
    # emits this instead of ``WasmOp.RETURN`` so ``catch`` absorbs the
    # unwind as TCL_RETURN rather than the WASM ``return`` jumping past
    # ``catch_leave`` and exiting the proc with the value.
    "tcl_return_set": ("tcl", "tcl_return_set", [ValType.I32], []),
    # 3-arg catch options dict.  ``catch BODY result opt`` populates
    # ``$opt`` with a dict carrying ``-code``, ``-level``, ``-errorcode``
    # and ``-errorinfo`` keys; ``dict get $opt -errorcode`` is the
    # standard idiom for inspecting the error class.
    "tcl_catch_options": ("tcl", "catch_options", [], [ValType.I32]),
    # 3-arg ``error msg ?info? ?code?`` form: lets the emitter
    # populate ``::errorInfo`` / ``::errorCode`` directly when the
    # source provides explicit info/code.  The 1-arg form keeps using
    # ``tcl_error`` (= ``tcl_cmd_error``) so existing emit sites
    # don't have to track which arity to call.
    "tcl_error_full": ("tcl", "tcl_cmd_error_full", [ValType.I32, ValType.I32, ValType.I32], []),
    # ``return -code error msg`` shortcut — equivalent to
    # ``tcl_cmd_error`` but ALSO sets the runtime flag that makes
    # the surrounding proc's epilogue skip the procedure-frame
    # stamp on errorInfo.  Mirrors Tcl 9's distinction between
    # body-level ``error msg`` (annotated by ``MakeProcError``)
    # and ``return -code error msg`` (bypasses ``MakeProcError``).
    # proc-old-7.2 checks the no-frame trace produced by this path.
    "tcl_error_via_return": ("tcl", "tcl_cmd_error_via_return", [ValType.I32], []),
    # Flow-control consumers — read+clear ``break_flag`` /
    # ``continue_flag`` set by interpreter-side ``break`` / ``continue``
    # inside an eval-fallback body.  Compiled loops need these to
    # propagate signals out of fallback regions; without the consume,
    # ``while 1 { dict update d k v { break } }`` would loop forever
    # because the wasm-side ``br`` only fires for compiled ``break``.
    "tcl_flow_consume_break": ("tcl", "flow_consume_break", [], [ValType.I32]),
    "tcl_flow_consume_continue": ("tcl", "flow_consume_continue", [], [ValType.I32]),
    # Pending-``return`` flag inspectors.  Compiled procs use these
    # at every callee / eval-fallback boundary to detect that
    # ``return`` (or ``return -code return``) raised from inside
    # the call.  ``check`` is non-destructive so the propagating
    # frame can decide whether to absorb (proc-dispatch boundary)
    # or pass through (mid-body eval-fallback).  ``take`` clears
    # the flag and yields the body's return value so the caller
    # frame can finish the absorption in one round-trip.
    "tcl_flow_check_return": ("tcl", "flow_check_return", [], [ValType.I32]),
    "tcl_flow_take_return": ("tcl", "flow_take_return", [], [ValType.I32]),
    "tcl_flow_check_signal_loop": ("tcl", "flow_check_signal_loop", [], [ValType.I32]),
    # Compiled ``for`` loops call this after the next-clause to
    # decide whether to iterate again.  Mirrors ``tclCmdAH.c``
    # ForPostNextCallback: BREAK collapses to TCL_OK + exit, while
    # CONTINUE / ERROR / RETURN exit the loop with their flag still
    # set so the for command's caller observes the original code.
    # See ``runtime/zig/interp/tcl_catch.zig::flow_for_next_post_check``.
    "tcl_flow_for_next_post_check": (
        "tcl",
        "flow_for_next_post_check",
        [],
        [ValType.I32],
    ),
    # Non-consuming "any control-flow flag pending?" probe.  Used
    # by compiled ``for`` after the init clause: init's BREAK /
    # CONTINUE / ERROR / RETURN propagate as the for command's
    # own exit code (matches ``tclCmdAH.c`` Tcl_ForObjCmd ~line
    # 2598), so we leave the flag set and only signal the emitter
    # to skip the loop entirely.
    "tcl_flow_check_any_signal": (
        "tcl",
        "flow_check_any_signal",
        [],
        [ValType.I32],
    ),
    # Interpreter fallback — every eval-path command routes through this.
    "tcl_eval": ("tcl", "tcl_eval", [ValType.I32], [ValType.I32]),
    # Runtime expression-source evaluator — used by the codegen's
    # ``ExprRaw`` path when an operand needs re-parsing as expr
    # source (e.g. backslash-substituted ``\\"…\\"`` from
    # ``[expr \\"X\\"]`` inside an outer ``"…"``).
    "tcl_eval_expr_str": (
        "tcl",
        "tcl_eval_expr_str",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # AOT strict logical-NOT — validates the operand is a
    # boolean (numeric or yes/no/true/false/on/off keyword),
    # raises ``cannot use non-numeric string "X" as operand of
    # "!"`` otherwise.  Used by ``UnaryOp.NOT`` in
    # ``_emit_unary`` so ``expr {!$v}`` for a non-boolean
    # ``$v`` errors instead of silently coercing through
    # ``obj_get_int``.
    "tcl_expr_lnot": (
        "tcl",
        "tcl_expr_lnot",
        [ValType.I32],
        [ValType.I32],
    ),
    # Namespace context for eval-fallback calls — compiled procs set the
    # current namespace before ``tcl_eval`` so dynamic ``proc $name`` /
    # ``variable $name`` inside the fallback qualify into the enclosing
    # namespace instead of global.  ``ns_set`` returns an opaque i64 save
    # token; ``ns_restore`` unwinds it.
    "tcl_ns_set": (
        "tcl",
        "ns_set",
        [ValType.I32, ValType.I32],
        [ValType.I64],
    ),
    "tcl_ns_restore": ("tcl", "ns_restore", [ValType.I64], []),
    # Frame stack (local variable scoping).
    "tcl_frame_push": ("tcl", "frame_push", [], [ValType.I32]),
    "tcl_frame_pop": ("tcl", "frame_pop", [], []),
    # Procedure-error frame stamp — called from the compiled-proc
    # epilogue before ``tcl_frame_pop``.  Adds ``(procedure "X" line
    # N)`` to ``::errorInfo`` when an error is pending.  Args:
    # ``(name_ptr, name_len)`` — the proc's fully-qualified name in
    # the WASM data section.
    "tcl_proc_stamp_error_frame": (
        "tcl",
        "proc_stamp_error_frame",
        [ValType.I32, ValType.I32],
        [],
    ),
    # Frame-side aliases — emitted by the compiled-proc prologue when
    # the body declares a ``variable X`` / ``global X`` so any
    # interpreter-side fallback (eval-script, dynamic ``while``,
    # ``foreach``, ``catch`` body, ...) sees the same alias the
    # compiled code uses.  Without this, the compiled proc reads /
    # writes ``X`` via ``tcl_global_get/set(target=::ns::X)`` while
    # the interpreter creates a fresh local ``X`` in the proc's
    # runtime frame -- two divergent views of the same variable name.
    "tcl_frame_alias_named": ("tcl", "frame_alias_named", [ValType.I32, ValType.I32], []),
    "tcl_frame_alias_global": ("tcl", "frame_alias_global", [ValType.I32], []),
    # ``upvar N other local`` / ``upvar #N other local`` — register
    # ``local`` in the current frame as an alias to variable ``other``
    # in the frame at absolute depth.  Reads/writes of the local then
    # resolve through ``tcl_local_get/set`` which transparently chase
    # the ALIAS_EXT bucket back to the target frame's variable.
    "tcl_frame_alias_frame_var": (
        "tcl",
        "frame_alias_frame_var",
        [ValType.I32, ValType.I32, ValType.I32],
        [],
    ),
    "tcl_frame_get_depth": ("tcl", "frame_get_depth", [], [ValType.I32]),
    # Resolve a dynamic ``upvar`` level token (``#N`` / ``N`` / ``$x``
    # post-substitution) to an absolute frame depth.  Mirrors the
    # parsing reference Tcl does in ``Tcl_UpVar2`` so OptTreeVars's
    # ``upvar $level $vname var`` (where ``$level`` is something like
    # ``"#3"``) compiles to a single helper call.
    "tcl_upvar_resolve_depth": (
        "tcl",
        "upvar_resolve_depth",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Static-relative ``upvar`` helper that walks back ``rel`` frames
    # belonging to the current interp, skipping frames pushed by
    # cross-interp alias / invokehidden trampolines.  Replaces the
    # inline ``frame_depth - rel`` emission so a hidden command's
    # ``upvar 1`` reaches its same-interp caller (interp-20.35 ..
    # 20.44).  Args: (cur_depth, rel).
    "tcl_upvar_walk_relative": (
        "tcl",
        "upvar_walk_relative",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Per-frame invocation argv — set by the compiled-proc prologue so
    # ``info level 0`` / ``info level -N`` inside the body reads the real
    # invocation list rather than a placeholder.
    "tcl_frame_set_argv": ("tcl", "frame_set_argv", [ValType.I32], []),
    "tcl_frame_get_argv": ("tcl", "frame_get_argv", [ValType.I32], [ValType.I32]),
    # Pending ``argv0`` slot — a compiled caller writes the exact word
    # it invoked the callee with immediately before the compiled
    # ``call``; the callee's prologue reads-and-clears it via
    # ``take_pending_argv0`` so ``info level 0`` reports the caller's
    # word (imported / renamed / qualified forms) rather than the
    # callee's registered qname tail.
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
    # Phase 8: per-frame call-site metadata for ``info frame``.
    # Stamped by the compiled-proc prologue so ``info frame N`` can
    # surface the proc name + body script for every active frame.
    # ``frame_set_type_i32`` takes 1=PROC, 2=SOURCE, 3=EVAL,
    # 4=UPLEVEL, 5=ALIAS, anything-else=UNKNOWN.
    "tcl_frame_set_type": ("tcl", "frame_set_type_i32", [ValType.I32], []),
    "tcl_frame_set_script": ("tcl", "frame_set_script", [ValType.I32], []),
    "tcl_frame_set_proc_name": ("tcl", "frame_set_proc_name", [ValType.I32], []),
    "tcl_frame_set_cmd_text": ("tcl", "frame_set_cmd_text", [ValType.I32], []),
    "tcl_frame_set_line": ("tcl", "frame_set_line", [ValType.I32], []),
    # Phase 8 follow-up: codegen claims line-stamp ownership for the
    # current frame so nested ``eval_script`` calls (resolving
    # ``[…]`` inside compiled bodies) don't overwrite per-statement
    # line stamps emitted by the prologue / body.
    "tcl_frame_claim_line_codegen": ("tcl", "frame_claim_line_codegen", [], []),
    # Phase 7: indexed local-variable accessors for codegen-resolved
    # compile-time-known scalar locals.  Bypasses the hash-keyed
    # store; per-frame slot capacity is ``LOCALS_ARRAY_CAP = 16``.
    "tcl_frame_local_at": ("tcl", "frame_local_at", [ValType.I32], [ValType.I32]),
    "tcl_frame_local_set_at": (
        "tcl",
        "frame_local_set_at",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Variable resolution (aliases, upvars, namespace vars).
    "tcl_var_resolve": ("tcl", "var_resolve", [ValType.I32], [ValType.I32]),
    "tcl_var_set": ("tcl", "var_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_var_exists": ("tcl", "var_exists", [ValType.I32], [ValType.I32]),
    "tcl_local_set": ("tcl", "local_set", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_local_get": ("tcl", "local_get", [ValType.I32], [ValType.I32]),
    # Strict variant of ``local_get`` — raises ``can't read "<name>":
    # no such variable`` when the frame slot has never been set.
    # Mirror of ``tcl_global_get_or_error`` for proc-local reads.
    "tcl_local_get_or_error": ("tcl", "local_get_or_error", [ValType.I32], [ValType.I32]),
    # Phase 6 follow-up: silent variants used by the codegen's
    # ``_emit_frame_sync`` / ``_emit_frame_readback`` paths.  They
    # share the value-store semantics of ``tcl_local_set`` /
    # ``tcl_local_get`` but bypass the variable-trace fire hooks —
    # frame syncs are state transfers between WASM-locals and the
    # frame's hash store, not user-visible assignments, so a
    # variable trace must not observe them.
    "tcl_local_set_silent": ("tcl", "local_set_silent", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_local_get_silent": ("tcl", "local_get_silent", [ValType.I32], [ValType.I32]),
    # Proc registry.
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
    # Source-text body stash for compiled procs.  Called from the
    # registration prologue right after ``proc_register_compiled`` so
    # ``info body`` can return the original ``proc`` body rather than
    # the empty string the AOT path otherwise leaves behind.
    "tcl_proc_set_body_source": (
        "tcl",
        "proc_set_body_source",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Stash the raw bytes of the params spec so ``info args`` /
    # ``info default`` can materialise a fresh TclObj on demand for
    # AOT-compiled procs.  Takes raw (name_ptr, name_len, params_ptr,
    # params_len) — the data-segment bytes are immutable so no TclObj
    # wrapper is needed, which keeps the per-proc init prologue
    # allocation-free for this sidecar.
    "tcl_proc_set_params_source_raw": (
        "tcl",
        "proc_set_params_source_raw",
        [ValType.I32, ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    # Info dispatch helpers (info exists / info <sub>).  The ``info``
    # command itself is a registry spec, but these two helpers are
    # invoked by the emitter directly rather than via a subcommand
    # table, so they live here.
    "tcl_info_exists": ("tcl", "info_exists", [ValType.I32], [ValType.I32]),
    "tcl_info_dispatch": ("tcl", "info_dispatch", [ValType.I32, ValType.I32], [ValType.I32]),
    # Frame-depth helpers for ``uplevel`` — temporarily shift frame_depth
    # so a called ``tcl_eval`` runs at a caller's scope.
    "tcl_frame_depth_stash": (
        "tcl",
        "frame_depth_stash",
        [ValType.I32],
        [ValType.I32],
    ),
    "tcl_frame_depth_stash_abs": (
        "tcl",
        "frame_depth_stash_abs",
        [ValType.I32],
        [ValType.I32],
    ),
    "tcl_frame_depth_restore": ("tcl", "frame_depth_restore", [ValType.I32], []),
    # Arrays — dedicated per-array hash tables.  ``array`` subcommands
    # are spec-owned but the helpers the codegen calls are shared
    # between ``array set`` literal emission and the sub-command
    # dispatcher, so they stay here.
    "tcl_array_set": (
        "tcl",
        "array_set",
        [ValType.I32, ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_get": ("tcl", "array_get", [ValType.I32, ValType.I32], [ValType.I32]),
    "tcl_array_get_all": (
        "tcl",
        "array_get_all",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_exists": ("tcl", "array_exists", [ValType.I32], [ValType.I32]),
    "tcl_array_element_exists": (
        "tcl",
        "array_element_exists",
        [ValType.I32, ValType.I32],
        [ValType.I32],
    ),
    "tcl_array_size": ("tcl", "array_size", [ValType.I32], [ValType.I32]),
    "tcl_array_unset": ("tcl", "array_unset", [ValType.I32], [ValType.I32]),
    # ``frame_resolve_array_name(local_name) -> resolved_name`` —
    # turns a bare proc-local name into the synthetic
    # ``::__local::<depth>::<name>`` directory key that the
    # eval-fallback's ``array set …`` already uses.  Compiled
    # ``_emit_array_name_obj`` calls this from inside a proc so
    # compiled and eval-fallback array operations agree on the
    # storage key (see :func:`_scan.scan_for_imports` for the
    # always-import rationale).
    "frame_resolve_array_name": (
        "tcl",
        "frame_resolve_array_name",
        [ValType.I32],
        [ValType.I32],
    ),
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
    # Diagnostic — record the current source site so trap paths can
    # prefix stderr with ``site=<id>`` for sidecar-map resolution.
    "tcl_diag_set": ("tcl", "diag_set", [ValType.I32], []),
}

# Import keys for the TclObj lifecycle functions — always registered
_OBJ_LIFECYCLE_IMPORTS = frozenset(
    {
        "tcl_obj_new_int",
        "tcl_obj_new_string",
        "tcl_obj_get_int",
        "tcl_obj_retain",
        "tcl_obj_release",
    }
)

# ``string is <class> <value>`` sub-sub-command → import key.
# Unlike the ordinary ``<command> <subcommand>`` dispatch (which lives
# on ``SubCommand.wasm_runtime_import``), this table dispatches on the
# *second* argument of ``string is`` — i.e. the character-class name.
# The registry doesn't model sub-sub-commands, so it stays as a dict.
_STRING_IS_IMPORT: dict[str, str] = {
    "integer": "tcl_string_is_integer",
    "wideinteger": "tcl_string_is_wideinteger",
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
