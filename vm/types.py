"""Core types for the Tcl VM: return codes, results, and errors."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum


class ReturnCode(IntEnum):
    """Tcl return codes matching the C-level TCL_OK / TCL_ERROR / etc."""

    OK = 0
    ERROR = 1
    RETURN = 2
    BREAK = 3
    CONTINUE = 4


@dataclass(slots=True)
class TclResult:
    """The result of evaluating a Tcl command or script."""

    code: ReturnCode = ReturnCode.OK
    value: str = ""
    error_info: list[str] = field(default_factory=list)
    error_code: str = "NONE"
    level: int = 0  # for ``return -level``


class TclError(Exception):
    """Runtime error raised inside the VM."""

    def __init__(
        self,
        message: str,
        error_code: str = "NONE",
        error_info: list[str] | None = None,
    ) -> None:
        super().__init__(message)
        self.message = message
        self.error_code = error_code
        self.error_info: list[str] = error_info or []


class TclReturn(Exception):
    """Raised to implement ``return`` across the Python call stack."""

    def __init__(
        self,
        value: str = "",
        code: int = 0,
        level: int = 1,
        error_info: str | None = None,
        error_code: str | None = None,
    ) -> None:
        super().__init__(value)
        self.value = value
        self.code = code
        self.level = level
        self.error_info = error_info
        self.error_code = error_code


class TclBreak(Exception):
    """Raised to implement ``break``."""


class TclContinue(Exception):
    """Raised to implement ``continue``."""


class TclCompletionError(Exception):
    """The input is syntactically incomplete (used by the REPL)."""


def _arith_error_from_exc(exc: Exception) -> TclError:
    """Convert a Python math exception to a TclError with the proper ARITH errorCode."""
    if isinstance(exc, ZeroDivisionError):
        msg = "divide by zero"
        return TclError(msg, error_code=f"ARITH DIVZERO {{{msg}}}")
    if isinstance(exc, OverflowError):
        msg = "floating-point value too large to represent"
        return TclError(msg, error_code=f"ARITH OVERFLOW {{{msg}}}")
    if isinstance(exc, ValueError):
        msg = "domain error: argument not in valid range"
        return TclError(msg, error_code=f"ARITH DOMAIN {{{msg}}}")
    return TclError(str(exc))
