"""WASM ``::tcl::mathop`` ensemble tests.

Covers the variadic / chain semantics that Tcl's ``mathop.n`` page
documents and that upstream tcltest fixtures (notably ``clock-7``'s
chained equality assertions) rely on.  Each test compiles a tiny Tcl
script, runs it under wasmtime, and asserts on captured stdout.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.strip()


@pytest.mark.parametrize(
    "expr,expected",
    [
        # Arithmetic
        ("::tcl::mathop::+ 1 2 3 4", "10"),
        ("::tcl::mathop::+", "0"),
        ("::tcl::mathop::- 10 1 2 3", "4"),
        ("::tcl::mathop::- 5", "-5"),
        ("::tcl::mathop::* 2 3 4", "24"),
        ("::tcl::mathop::*", "1"),
        ("::tcl::mathop::/ 100 5 2", "10"),
        ("::tcl::mathop::% 17 5", "2"),
        ("::tcl::mathop::** 2 3", "8"),
        ("::tcl::mathop::** 2 3 2", "512"),  # right-assoc: 2 ** (3 ** 2) = 2 ** 9
        # Chained comparisons (clock-7's idiom)
        ("::tcl::mathop::== 1 1 1", "1"),
        ("::tcl::mathop::== 1 1 2", "0"),
        ("::tcl::mathop::< 1 2 3 4", "1"),
        ("::tcl::mathop::< 1 2 2 4", "0"),
        ("::tcl::mathop::<= 1 2 2 4", "1"),
        ("::tcl::mathop::> 4 3 2 1", "1"),
        ("::tcl::mathop::>= 4 4 3 1", "1"),
        ("::tcl::mathop::!= 1 2", "1"),
        # String compare
        ("::tcl::mathop::eq abc abc abc", "1"),
        ("::tcl::mathop::eq abc abd", "0"),
        ("::tcl::mathop::ne abc def", "1"),
        # Bitwise
        ("::tcl::mathop::& 12 10", "8"),
        ("::tcl::mathop::| 12 10", "14"),
        ("::tcl::mathop::^ 12 10", "6"),
        ("::tcl::mathop::~ 0", "-1"),
        ("::tcl::mathop::<< 1 4", "16"),
        ("::tcl::mathop::>> 16 2", "4"),
        # Logical
        ("::tcl::mathop::! 0", "1"),
        ("::tcl::mathop::! 1", "0"),
        ("::tcl::mathop::&& 1 1 1", "1"),
        ("::tcl::mathop::&& 1 0 1", "0"),
        ("::tcl::mathop::|| 0 0 1", "1"),
        ("::tcl::mathop::|| 0 0 0", "0"),
        # Min / max
        ("::tcl::mathop::min 5 2 7 3", "2"),
        ("::tcl::mathop::max 5 2 7 3", "7"),
        # List membership
        ("::tcl::mathop::in b {a b c}", "1"),
        ("::tcl::mathop::in z {a b c}", "0"),
        ("::tcl::mathop::ni z {a b c}", "1"),
    ],
)
def test_mathop_op(expr: str, expected: str) -> None:
    out = _run(f"puts [{expr}]")
    assert out == expected, f"{expr!r}: expected {expected!r}, got {out!r}"


def test_mathop_float_arithmetic() -> None:
    # Floating-point results should keep ``.`` so Tcl recognises them.
    out = _run("puts [::tcl::mathop::+ 1.5 2.5]")
    assert out in {"4.0", "4"}  # depending on canonicalisation


def test_mathop_short_qualified_form() -> None:
    # The half-qualified ``tcl::mathop::==`` form (no leading ``::``)
    # must also resolve — used inside ``namespace eval ::tcl`` blocks.
    out = _run("puts [tcl::mathop::== 1 1 1]")
    assert out == "1"


def test_mathop_chain_under_one_arg_returns_true() -> None:
    # Tcl's chain-comparison ops return 1 vacuously for 0 / 1 args.
    assert _run("puts [::tcl::mathop::< 5]") == "1"
    assert _run("puts [::tcl::mathop::<]") == "1"


def test_mathop_div_unary_returns_floating_reciprocal() -> None:
    # ``mathop.n``: with one argument, ``/`` returns ``1.0/x``.  An
    # integer input still produces a *float* — ``[/ 5]`` is ``0.2``,
    # not ``0`` (integer floor).  Codex P1 review caught the
    # earlier integer-floor implementation.
    assert _run("puts [::tcl::mathop::/ 5]") == "0.2"
    assert _run("puts [::tcl::mathop::/ 4]") == "0.25"
    # Unary on a float input keeps the float path.
    assert _run("puts [::tcl::mathop::/ 2.0]") == "0.5"
    # Multi-arg integer division still floors as before.
    assert _run("puts [::tcl::mathop::/ 100 5 2]") == "10"


def test_mathop_compare_falls_back_to_string_for_non_numeric() -> None:
    # ``mathop`` comparison ops compare numerically when both args
    # are numeric, otherwise lexically.  Without this fallback,
    # ``obj_get_int("a")`` collapsed every non-numeric string to
    # ``0`` and every comparison returned vacuously true.
    assert _run("puts [::tcl::mathop::== a b]") == "0"
    assert _run("puts [::tcl::mathop::== abc abc]") == "1"
    assert _run("puts [::tcl::mathop::!= a b]") == "1"
    assert _run("puts [::tcl::mathop::< a b]") == "1"
    assert _run("puts [::tcl::mathop::> b a]") == "1"
    assert _run("puts [::tcl::mathop::<= ab ac]") == "1"
    assert _run("puts [::tcl::mathop::>= ab ab]") == "1"
    # Mixed numeric / string falls back to string compare too —
    # ``"10"`` lexically precedes ``"9"`` because ``'1' < '9'``.
    assert _run("puts [::tcl::mathop::< 10 9z]") == "1"
    # Pure numeric still goes through the numeric path.
    assert _run("puts [::tcl::mathop::== 1 1.0]") == "1"
    assert _run("puts [::tcl::mathop::< 9 10]") == "1"


def test_mathop_div_negative_floors() -> None:
    # ``::tcl::mathop::/`` is the same operator as ``expr {a / b}`` and
    # must match it: Tcl 9 integer division rounds toward negative
    # infinity (floored), so ``5 / -2`` is ``-3`` (not the truncated
    # ``-2``).  The command form now folds over ``tcl_arith_div``
    # (which floors), unifying it with the expression compiler.
    assert _run("puts [::tcl::mathop::/ 5 -2]") == "-3"
    assert _run("puts [::tcl::mathop::/ -5 2]") == "-3"
    assert _run("puts [::tcl::mathop::/ -7 -2]") == "3"


def test_mathop_mod_uses_floor_sign_of_divisor() -> None:
    # ``::tcl::mathop::%`` matches Tcl 9 floor-mod semantics
    # (``tclExecute.c`` INST_MOD): the result has the *divisor's*
    # sign.  The pre-bignum ``@rem``-sign-of-dividend path made
    # ``[% -7 3]`` return ``-1``, miscompiled w.r.t. C Tcl which
    # returns ``2``.  ``tcl_arith_mod`` (and thus mathop's ``%``)
    # now apply the ``r += b when signs differ`` fixup upstream
    # uses, so all three of these match the C Tcl reference.
    assert _run("puts [::tcl::mathop::% -7 3]") == "2"
    assert _run("puts [::tcl::mathop::% 7 -3]") == "-2"
    assert _run("puts [::tcl::mathop::% -7 -3]") == "-1"


def test_mathop_pow_negative_exponent_is_integer() -> None:
    # Integer base ** negative integer exponent stays on the INTEGER
    # path in Tcl 9 (matching the ``**`` operator and mathop-25.15):
    # ``|base| > 1`` truncates the fractional reciprocal to ``0``,
    # ``base == 1`` is ``1``, and ``base == -1`` follows the exponent
    # parity.  (A *float* exponent — ``** 2 -1.0`` — would promote.)
    assert _run("puts [::tcl::mathop::** 2 -1]") == "0"
    assert _run("puts [::tcl::mathop::** 4 -2]") == "0"
    assert _run("puts [::tcl::mathop::** 1 -5]") == "1"
    assert _run("puts [::tcl::mathop::** -1 -3]") == "-1"
    assert _run("puts [::tcl::mathop::** -1 -2]") == "1"
    # Positive integer exponents stay on the integer path.
    assert _run("puts [::tcl::mathop::** 3 4]") == "81"


def test_mathop_logical_boolean_keywords() -> None:
    # ``!`` / ``&&`` / ``||`` must accept Tcl's full boolean
    # keyword set (``true`` / ``false`` / ``yes`` / ``no`` /
    # ``on`` / ``off`` and their non-ambiguous prefixes), not the
    # earlier 1-or-2-character prefix heuristic that misclassified
    # strings like ``tree`` / ``frame`` as boolean (Copilot review).
    assert _run("puts [::tcl::mathop::! true]") == "0"
    assert _run("puts [::tcl::mathop::! yes]") == "0"
    assert _run("puts [::tcl::mathop::! on]") == "0"
    assert _run("puts [::tcl::mathop::! false]") == "1"
    assert _run("puts [::tcl::mathop::! no]") == "1"
    assert _run("puts [::tcl::mathop::! off]") == "1"
    # Ambiguous prefixes that LOOK like a boolean keyword
    # — ``"tree"`` / ``"frame"`` are neither boolean keywords nor
    # numbers, so ``!`` raises ``cannot use non-numeric string "X" as
    # operand of "!"`` (mathop-21.5), matching reference Tcl.  (The
    # old prefix heuristic silently treated them as ``0`` → ``1``.)
    assert (
        _run('puts [catch {::tcl::mathop::! tree} m]:$m')
        == '1:cannot use non-numeric string "tree" as operand of "!"'
    )
    assert (
        _run('puts [catch {::tcl::mathop::! frame} m]:$m')
        == '1:cannot use non-numeric string "frame" as operand of "!"'
    )
    # Numeric truth still works.
    assert _run("puts [::tcl::mathop::&& 1 1 1]") == "1"
    assert _run("puts [::tcl::mathop::|| 0 0 1]") == "1"


# -- ``namespace import ::tcl::mathop::*`` materialisation -----------------
#
# The ``::tcl::mathop`` ensemble's commands live in the static BUILTINS
# slice keyed by FQN (``::tcl::mathop::+`` and friends), not in any
# namespace's ``cmd_table``.  Until the runtime materialises the
# ensemble lazily via ``dispatch/tcl_builtin_ns.zig``, ``namespace
# import ::tcl::mathop::*`` raises ``unknown namespace in import
# pattern "::tcl::mathop::*"`` because the existence check at
# ``cmds/namespace.zig`` doesn't know that the BUILTINS slice implies
# the namespace exists.  ``mathop.test`` (Tcl 9 core slice) is the
# canonical caller and trapped at line 22 before our fix.


def test_namespace_import_mathop_glob() -> None:
    """``namespace import ::tcl::mathop::*`` must succeed and pull every
    operator into the destination namespace, where the bare names
    dispatch through the import redirect to the BUILTIN handler."""
    out = _run(
        """
namespace eval ::testmathop2 {
    namespace import ::tcl::mathop::*
}
puts [::testmathop2::+ 1 2 3]
puts [::testmathop2::== 1 1 1]
puts [::testmathop2::eq abc abc abc]
puts [::testmathop2::min 5 2 7 3]
puts [::testmathop2::in b {a b c}]
"""
    )
    assert out == "6\n1\n1\n2\n1"


def test_namespace_import_mathop_single_name() -> None:
    """A targeted ``namespace import ::tcl::mathop::min`` (no glob) must
    also materialise the source namespace and import just the named
    op."""
    out = _run(
        """
namespace eval ::picker {
    namespace import ::tcl::mathop::min
    namespace import ::tcl::mathop::max
}
puts [::picker::min 5 2 7]
puts [::picker::max 5 2 7]
"""
    )
    assert out == "2\n7"


def test_info_commands_lists_mathop_ensemble() -> None:
    """``info commands ::tcl::mathop::*`` must enumerate every operator
    even before any other code touches the ensemble — the
    ``info commands`` walker materialises BUILTIN namespaces lazily."""
    out = _run("puts [llength [info commands ::tcl::mathop::*]]")
    assert out == "32"


def test_materialise_only_for_builtin_backed_namespaces() -> None:
    """Materialisation must only fire for namespaces backed by a
    BUILTINS prefix.  An unknown prefix must remain unknown — i.e.
    ``namespace exists`` against a name that has no BUILTINS entries
    must still return 0 — so the materialise hook isn't degrading
    into "any qualified ns is fine"."""
    out = _run("puts [namespace exists ::nope::does_not_exist]")
    assert out == "0"


def test_materialise_does_not_create_unrelated_namespaces() -> None:
    """After ``namespace import`` triggers materialisation of
    ``::tcl::mathop``, a sibling whose name shares the prefix but
    isn't backed by BUILTINS (``::tcl::mathop_BOGUS``) must NOT
    spring into existence as a side-effect."""
    out = _run(
        """
namespace import ::tcl::mathop::*
puts [namespace exists ::tcl::mathop]
puts [namespace exists ::tcl::mathop_BOGUS]
"""
    )
    assert out == "1\n0"


def test_namespace_import_mathop_partial_glob() -> None:
    """The trailing simple-name component of an import spec is matched
    by the standard Tcl ``string match`` glob — ``m*`` brings in
    ``min``/``max`` only, not the rest of the ensemble."""
    out = _run(
        """
namespace eval ::partial {
    namespace import ::tcl::mathop::m*
}
puts [lsort [info commands ::partial::*]]
puts [::partial::min 5 2 7]
puts [::partial::max 5 2 7]
"""
    )
    # Qualified pattern ``::partial::*`` returns FQ names from the
    # queried ns (info-4.x — qualified patterns preserve their
    # qualifiers in the output).
    assert out == "::partial::max ::partial::min\n2\n7"


def test_namespace_path_through_eval_materialises_mathop() -> None:
    """When ``namespace path`` is dispatched at runtime (e.g. via
    ``eval``), the path-setter hook in ``cmds/namespace.zig``
    materialises the target.  The bare top-level form is currently
    elided by codegen as a no-op, so this test goes through ``eval``
    to exercise the runtime path that the hook actually owns."""
    out = _run(
        """
puts [namespace exists ::tcl::mathop]
eval namespace path ::tcl::mathop
puts [namespace exists ::tcl::mathop]
"""
    )
    # 0 (cold) → 1 (after path).
    assert out == "0\n1"


def test_info_commands_after_explicit_namespace_eval() -> None:
    """If the user has pre-created the BUILTIN namespace explicitly
    (``namespace eval ::tcl::mathop {}``) without populating it,
    ``info commands ::tcl::mathop::*`` must still enumerate the
    operators — verifies the unconditional materialise hook in
    ``tcl_cmd_info.zig`` (was previously gated on
    ``qualified_target == 0``)."""
    out = _run(
        """
namespace eval ::tcl::mathop {}
puts [llength [info commands ::tcl::mathop::*]]
"""
    )
    assert out == "32"


def test_mathfunc_namespace_materialises_too() -> None:
    """The helper isn't mathop-specific.  ``::tcl::mathfunc``'s
    BUILTIN entries (``::tcl::mathfunc::sin`` etc.) must also
    materialise on demand."""
    out = _run(
        """
puts [namespace exists ::tcl::mathfunc]
puts [llength [info commands ::tcl::mathfunc::*]]
puts [namespace exists ::tcl::mathfunc]
"""
    )
    # Cold ``namespace exists`` returns 0; ``info commands`` triggers
    # materialise; subsequent ``namespace exists`` is 1.  The exact
    # count of mathfunc entries is registry-dependent, so just
    # require "more than zero".
    parts = out.split("\n")
    assert parts[0] == "0", f"::tcl::mathfunc must not pre-exist before lookup: got {parts[0]!r}"
    assert int(parts[1]) > 0, (
        f"info commands ::tcl::mathfunc::* must enumerate after materialise: got {parts[1]!r}"
    )
    assert parts[2] == "1", f"::tcl::mathfunc must exist after materialise: got {parts[2]!r}"


def test_materialise_idempotent_across_two_destinations() -> None:
    """Importing the same source ensemble into two different
    destinations must succeed for both — verifies that
    ``materialise``'s "skip slots already populated as forwards"
    path is a true no-op the second time around (no clobber, no
    error)."""
    out = _run(
        """
namespace eval ::a {
    namespace import ::tcl::mathop::*
}
namespace eval ::b {
    namespace import ::tcl::mathop::*
}
puts [::a::+ 1 2 3]
puts [::b::+ 10 20 30]
"""
    )
    assert out == "6\n60"


def test_materialise_from_non_root_ns_does_not_pollute_tree() -> None:
    """``register_builtin_forward`` resolves non-``::``-prefixed
    names relative to ``ns_current()``.  ``cmds/tcl_mathop.zig``
    registers each operator under both fully-qualified
    (``::tcl::mathop::+``) and half-qualified (``tcl::mathop::+``)
    spellings.  Without the ``::``-prefix filter in ``materialise``,
    invoking the helper while ``ns_current`` is non-root would walk
    the half-qualified names through the relative resolver and
    create ``::caller::tcl::mathop::+`` etc., leaving the real
    ``::tcl::mathop`` empty and polluting the caller's subtree.
    Verify both halves: the canonical ns is populated, and no
    ``tcl`` child appears under the caller's ns."""
    out = _run(
        """
namespace eval ::caller {
    # Force materialisation from inside ::caller — info commands
    # is the cheapest reachable hook.
    set _ [info commands ::tcl::mathop::*]
    puts [namespace exists ::caller::tcl::mathop]
    puts [namespace exists ::caller::tcl]
    puts [namespace exists ::tcl::mathop]
}
"""
    )
    # No phantom ``::caller::tcl::mathop`` (or even ``::caller::tcl``);
    # the canonical ``::tcl::mathop`` is the only target.
    assert out == "0\n0\n1"


def test_namespace_import_after_explicit_namespace_eval() -> None:
    """A user that has already opened the source namespace explicitly
    (``namespace eval ::tcl::mathop {}``) without populating it must
    still see the BUILTINS forwards when they later import — the
    materialiser fires unconditionally so the import isn't silently
    a no-op against an empty cmd_table."""
    out = _run(
        """
# Pre-create the ns the user-visible way (empty body).  Without
# unconditional materialisation, the import below would walk an
# empty cmd_table and bring nothing across.
namespace eval ::tcl::mathop {}
namespace eval ::dst {
    namespace import ::tcl::mathop::*
}
puts [::dst::+ 1 2 3]
puts [::dst::== 1 1 1]
"""
    )
    assert out == "6\n1"


# -- Large-shift contract (mirrors tclExecute.c INST_LSHIFT / INST_RSHIFT) --
#
# Tcl 9 caps shift counts at INT_MAX (2^31 - 1).  ``[<< 1 (INT_MAX+1)]``
# would otherwise demand a 2^31-bit bignum, which we can't materialise
# in finite time, so the runtime must reject the count *before* calling
# into ``Managed.shiftLeft`` / ``Managed.shiftRight``.  Pre-fix the
# ``mathop.test`` slice (specifically ``mathop-24.5``) hung the WASM
# bundle for the full 180 s watchdog window.
#
# These contracts pin both:
#   - left-shift count > INT_MAX → "integer value too large to represent"
#   - right-shift count > INT_MAX → 0 / -1 per operand sign
#   - bignum-shaped count is treated identically to a count > INT_MAX


def test_mathop_lshift_count_above_int_max_errors() -> None:
    """``[<< 1 2147483648]`` (INT_MAX + 1) must raise
    ``integer value too large to represent`` rather than allocate a
    2-billion-bit bignum and hang the runtime."""
    out = _run(
        """
if {[catch {::tcl::mathop::<< 1 2147483648} msg]} {
    puts $msg
} else {
    puts "no-error"
}
"""
    )
    assert out == "integer value too large to represent"


def test_mathop_lshift_count_far_above_int_max_errors() -> None:
    """Same contract for a count that exceeds 2^32 — proves the cap is
    INT_MAX, not the i32 wraparound boundary."""
    out = _run(
        """
if {[catch {::tcl::mathop::<< 1 4294967296} msg]} {
    puts $msg
}
"""
    )
    assert out == "integer value too large to represent"


def test_mathop_lshift_count_bignum_errors() -> None:
    """A shift count that's a positive bignum (well past i64) must also
    surface as ``integer value too large to represent`` — without
    silently truncating to the low-64 bits and accepting the shift."""
    out = _run(
        """
set big 12135435435354435435342423948763867876
if {[catch {::tcl::mathop::<< 1 $big} msg]} {
    puts $msg
}
"""
    )
    assert out == "integer value too large to represent"


def test_mathop_lshift_negative_bignum_count_errors() -> None:
    """A negative shift count (even when expressed as a bignum) keeps
    the ``negative shift argument`` error wording."""
    out = _run(
        """
set big -12135435435354435435342423948763867876
if {[catch {::tcl::mathop::<< 1 $big} msg]} {
    puts $msg
}
"""
    )
    assert out == "negative shift argument"


def test_mathop_lshift_count_at_int_max_succeeds() -> None:
    """The boundary itself (count == INT_MAX) is still legal; only
    counts strictly greater than INT_MAX are rejected.  We use a
    zero operand so the result is trivially 0 and we don't actually
    materialise a 2^31-bit value."""
    out = _run("puts [::tcl::mathop::<< 0 2147483647]")
    assert out == "0"


def test_mathop_rshift_count_above_int_max_returns_zero() -> None:
    """``[>> $positive INT_MAX+1]`` collapses to 0 rather than feeding
    an absurd shift count to ``Managed.shiftRight``."""
    out = _run("puts [::tcl::mathop::>> 12345 2147483648]")
    assert out == "0"


def test_mathop_rshift_count_above_int_max_negative_operand_returns_minus_one() -> None:
    """Mirror of the above for arithmetic-shift sign extension."""
    out = _run("puts [::tcl::mathop::>> -12345 2147483648]")
    assert out == "-1"


def test_mathop_rshift_bignum_operand_count_above_int_max() -> None:
    """A bignum operand right-shifted by > INT_MAX still inspects the
    operand's true sign, not the truncated low-64 bits — so a large
    *positive* bignum collapses to 0 and a large *negative* bignum
    collapses to -1."""
    out = _run(
        """
set big      12135435435354435435342423948763867876
set wide                             12345678912345
puts [::tcl::mathop::>> $big $wide]
puts [::tcl::mathop::>> -$big $wide]
"""
    )
    assert out == "0\n-1"


def test_expr_lshift_count_above_int_max_errors() -> None:
    """The same cap applies inside ``expr`` — ``tcl_arith_lshift``
    shares the contract with ``::tcl::mathop::<<``."""
    out = _run(
        """
if {[catch {expr {1 << 2147483648}} msg]} {
    puts $msg
}
"""
    )
    assert out == "integer value too large to represent"


def test_expr_rshift_count_above_int_max_collapses() -> None:
    """``expr {12345 >> (INT_MAX+1)}`` collapses to 0; the negative
    operand counterpart collapses to -1."""
    assert _run("puts [expr {12345 >> 2147483648}]") == "0"
    assert _run("puts [expr {-12345 >> 2147483648}]") == "-1"
