"""Math-related commands: incr, and tcl::mathfunc wrappers."""

from __future__ import annotations

import math
import random
from typing import TYPE_CHECKING

from ..machine import _format_number, _parse_int_strict, _parse_number
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_incr(interp: TclInterp, args: list[str]) -> TclResult:
    """incr varName ?increment?"""
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "incr varName ?increment?"')
    name = args[0]
    amount = _parse_int_strict(args[1]) if len(args) > 1 else 1
    old = _parse_int_strict(interp.current_frame.get_var(name, default="0"))
    result = str(old + amount)
    interp.current_frame.set_var(name, result)
    return TclResult(value=result)


# tcl::mathfunc::* wrappers
# Invoked via ``invokeStk`` when the compiler inlines math function
# calls in expressions (matching tclsh 9.0 codegen).


def _mathfunc(fn, *, min_args: int = 1, max_args: int = 1):  # noqa: ANN001, ANN202
    """Wrap a Python callable as a tcl::mathfunc command."""
    # Extract function name from _mf_<name> convention
    func_name = fn.__name__[4:] if fn.__name__.startswith("_mf_") else fn.__name__

    def handler(_interp: TclInterp, args: list[str]) -> TclResult:
        nargs = len(args)
        if nargs < min_args:
            raise TclError(f'not enough arguments for math function "{func_name}"')
        if max_args >= 0 and nargs > max_args:
            raise TclError(f'too many arguments for math function "{func_name}"')
        try:
            return TclResult(value=fn(args))
        except ValueError as exc:
            # Check if the error is from a non-numeric argument
            exc_str = str(exc)
            if "could not convert string to float" in exc_str:
                for a in args:
                    try:
                        float(a)
                    except (ValueError, TypeError):
                        if " " in a.strip():
                            raise TclError(
                                "expected floating-point number but got a list"
                            ) from None
                        raise TclError(f'expected floating-point number but got "{a}"') from None
            from ..types import _arith_error_from_exc

            raise _arith_error_from_exc(exc) from None
        except OverflowError as exc:
            from ..types import _arith_error_from_exc

            raise _arith_error_from_exc(exc) from None
        except ZeroDivisionError as exc:
            from ..types import _arith_error_from_exc

            raise _arith_error_from_exc(exc) from None
        except (TypeError, IndexError) as exc:
            raise TclError(str(exc)) from None

    return handler


def _mf_abs(args: list[str]) -> str:
    return _format_number(abs(_parse_number(args[0])))


def _mf_acos(args: list[str]) -> str:
    v = float(args[0])
    if math.isnan(v):
        raise ValueError("domain error: argument not in valid range")
    return _format_number(math.acos(v))


def _mf_asin(args: list[str]) -> str:
    return _format_number(math.asin(float(args[0])))


def _mf_atan(args: list[str]) -> str:
    return _format_number(math.atan(float(args[0])))


def _mf_atan2(args: list[str]) -> str:
    return _format_number(math.atan2(float(args[0]), float(args[1])))


def _mf_bool(args: list[str]) -> str:
    from ..interp import _tcl_bool

    return "1" if _tcl_bool(args[0]) else "0"


def _mf_ceil(args: list[str]) -> str:
    return _format_number(int(math.ceil(float(args[0]))))


def _mf_cos(args: list[str]) -> str:
    return _format_number(math.cos(float(args[0])))


def _mf_cosh(args: list[str]) -> str:
    try:
        return _format_number(math.cosh(float(args[0])))
    except OverflowError:
        return "Inf"


def _mf_double(args: list[str]) -> str:
    return _format_number(float(args[0]))


def _mf_entier(args: list[str]) -> str:
    return str(int(float(args[0])))


def _mf_exp(args: list[str]) -> str:
    try:
        return _format_number(math.exp(float(args[0])))
    except OverflowError:
        # IEEE: return Inf for overflow
        return "Inf" if float(args[0]) > 0 else "0.0"


def _mf_floor(args: list[str]) -> str:
    return _format_number(int(math.floor(float(args[0]))))


def _mf_fmod(args: list[str]) -> str:
    return _format_number(math.fmod(float(args[0]), float(args[1])))


def _mf_hypot(args: list[str]) -> str:
    return _format_number(math.hypot(float(args[0]), float(args[1])))


def _mf_int(args: list[str]) -> str:
    return str(int(float(args[0])))


def _mf_isqrt(args: list[str]) -> str:
    return str(math.isqrt(int(args[0])))


def _mf_log(args: list[str]) -> str:
    return _format_number(math.log(float(args[0])))


def _mf_log10(args: list[str]) -> str:
    return _format_number(math.log10(float(args[0])))


def _mf_max(args: list[str]) -> str:
    vals = [_parse_number(v) for v in args]
    result = max(vals)
    # For integers, return the exact integer string (not _format_number
    # which would convert huge ints to float Inf)
    if isinstance(result, int):
        return str(result)
    return _format_number(result)


def _mf_min(args: list[str]) -> str:
    vals = [_parse_number(v) for v in args]
    result = min(vals)
    if isinstance(result, int):
        return str(result)
    return _format_number(result)


def _mf_pow(args: list[str]) -> str:
    try:
        return _format_number(math.pow(float(args[0]), float(args[1])))
    except OverflowError:
        # IEEE platforms return Inf/-Inf for overflow
        base = float(args[0])
        exp = float(args[1])
        if base < 0 and exp == int(exp) and int(exp) % 2 == 1:
            return "-Inf"
        return "Inf"


def _mf_round(args: list[str]) -> str:
    v = float(args[0])
    if v >= 0:
        return str(int(math.floor(v + 0.5)))
    return str(int(math.ceil(v - 0.5)))


def _mf_sin(args: list[str]) -> str:
    return _format_number(math.sin(float(args[0])))


def _mf_sinh(args: list[str]) -> str:
    try:
        return _format_number(math.sinh(float(args[0])))
    except OverflowError:
        return "-Inf" if float(args[0]) < 0 else "Inf"


def _mf_sqrt(args: list[str]) -> str:
    return _format_number(math.sqrt(float(args[0])))


def _mf_tan(args: list[str]) -> str:
    return _format_number(math.tan(float(args[0])))


def _mf_tanh(args: list[str]) -> str:
    return _format_number(math.tanh(float(args[0])))


def _mf_rand(args: list[str]) -> str:
    return _format_number(random.random())


def _mf_srand(args: list[str]) -> str:
    random.seed(int(args[0]))
    return _format_number(random.random())


def _mf_wide(args: list[str]) -> str:
    v = args[0].strip()
    try:
        return str(int(v, 0))
    except (ValueError, TypeError):
        pass
    try:
        return str(int(v, 10))
    except (ValueError, TypeError):
        pass
    # Float to wide int: truncate like C's (Tcl_WideInt)(double)

    fv = float(v)
    # Pack as C double, unpack as signed 64-bit int equivalent via
    # C-style truncation.  For values in range, int() suffices.
    # For out-of-range values, use struct to get the C behaviour.
    try:
        iv = int(fv)
        # Clamp to 64-bit signed range (C behaviour)
        if iv < -(2**63) or iv >= 2**63:
            # Wrap like C: pack as unsigned, read as signed
            iv = iv % (2**64)
            if iv >= 2**63:
                iv -= 2**64
        return str(iv)
    except (ValueError, OverflowError):
        raise ValueError(f'expected integer but got "{args[0]}"') from None


# (function, min_args, max_args)  — max_args=-1 means variadic
_MATH_FUNCS: dict[str, tuple[object, int, int]] = {
    "abs": (_mf_abs, 1, 1),
    "acos": (_mf_acos, 1, 1),
    "asin": (_mf_asin, 1, 1),
    "atan": (_mf_atan, 1, 1),
    "atan2": (_mf_atan2, 2, 2),
    "bool": (_mf_bool, 1, 1),
    "ceil": (_mf_ceil, 1, 1),
    "cos": (_mf_cos, 1, 1),
    "cosh": (_mf_cosh, 1, 1),
    "double": (_mf_double, 1, 1),
    "entier": (_mf_entier, 1, 1),
    "exp": (_mf_exp, 1, 1),
    "floor": (_mf_floor, 1, 1),
    "fmod": (_mf_fmod, 2, 2),
    "hypot": (_mf_hypot, 2, 2),
    "int": (_mf_int, 1, 1),
    "isqrt": (_mf_isqrt, 1, 1),
    "log": (_mf_log, 1, 1),
    "log10": (_mf_log10, 1, 1),
    "max": (_mf_max, 1, -1),
    "min": (_mf_min, 1, -1),
    "pow": (_mf_pow, 2, 2),
    "rand": (_mf_rand, 0, 0),
    "round": (_mf_round, 1, 1),
    "sin": (_mf_sin, 1, 1),
    "sinh": (_mf_sinh, 1, 1),
    "srand": (_mf_srand, 1, 1),
    "sqrt": (_mf_sqrt, 1, 1),
    "tan": (_mf_tan, 1, 1),
    "tanh": (_mf_tanh, 1, 1),
    "wide": (_mf_wide, 1, 1),
}


def register() -> None:
    """Register math commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("incr", _cmd_incr)
    for name, (fn, lo, hi) in _MATH_FUNCS.items():
        REGISTRY.register_handler(f"tcl::mathfunc::{name}", _mathfunc(fn, min_args=lo, max_args=hi))
