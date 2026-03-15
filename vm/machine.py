"""Bytecode stack machine for the Tcl VM.

Interprets ``FunctionAsm`` instruction streams produced by the codegen.
All values on the stack are Python ``str`` (Tcl's everything-is-a-string).
"""

from __future__ import annotations

import math
import re
from dataclasses import dataclass
from typing import TYPE_CHECKING

from core.compiler.codegen import FunctionAsm, Instruction, Op
from core.compiler.codegen.opcodes import _INDEX_END

from .compiler import _BRACE_CLOSE, _BRACE_OPEN, _RAW_PREFIX
from .types import ReturnCode, TclBreak, TclContinue, TclError, TclResult, TclReturn

if TYPE_CHECKING:
    from .interp import TclInterp


# Loop region analysis


@dataclass(slots=True)
class _LoopRegion:
    """Describes a loop region in the bytecode (detected from backward jumps)."""

    header_idx: int  # instruction index of the loop header
    exit_idx: int  # instruction index to jump to on break
    step_idx: int  # instruction index to jump to on continue
    back_jump_idx: int  # instruction index of the backward jump


def _find_loops(
    instrs: list[Instruction],
    offset_map: dict[int, int],
    labels: dict[str, int],
) -> list[_LoopRegion]:
    """Pre-analyse bytecode to find loop structures from backward jumps.

    Detects both top-tested loops (unconditional backward jump from body
    to header) and bottom-tested loops (conditional backward jump as the
    loop back-edge).
    """
    loops: list[_LoopRegion] = []

    # Collect step-block offsets from label names (for_step_*, while has none)
    step_offsets: list[int] = [offset for label, offset in labels.items() if "_step_" in label]

    _JUMP_OPS = (Op.JUMP1, Op.JUMP4)
    _COND_JUMP_OPS = (
        Op.JUMP_TRUE1,
        Op.JUMP_TRUE4,
        Op.JUMP_FALSE1,
        Op.JUMP_FALSE4,
    )

    for i, instr in enumerate(instrs):
        is_uncond = instr.op in _JUMP_OPS
        is_cond = instr.op in _COND_JUMP_OPS
        if not is_uncond and not is_cond:
            continue

        target_op = instr.operands[0]
        if isinstance(target_op, str):
            target_offset = labels.get(target_op, -1)
        else:
            target_offset = target_op
        target_idx = offset_map.get(target_offset)
        if target_idx is None or target_idx >= i:
            continue  # Forward jump, not a loop

        back_jump_offset = instr.offset

        if is_uncond:
            # Unconditional backward jump.  With bottom-tested loop
            # reordering, normal loops use a conditional back-edge
            # (jumpTrue); an unconditional backward jump means the
            # loop condition was constant-folded away (e.g. while {1}).
            # The only exit is via break — set exit to the instruction
            # after this backward jump.
            header_idx = target_idx
            exit_idx = i + 1

        else:
            # Bottom-tested loop: conditional backward jump is the back-edge.
            # The body starts at the target; exit is the instruction after
            # this conditional jump.
            header_idx = target_idx
            exit_idx = i + 1

        # Find step block: pick the step label closest to the backward
        # jump within this loop's range (avoids matching an outer loop's
        # step when loops are nested).
        step_idx = header_idx  # default: no step (while loop)
        best_step_off = -1
        for soff in step_offsets:
            if target_offset < soff < back_jump_offset:
                if soff > best_step_off:
                    best_step_off = soff
        if best_step_off >= 0:
            sidx = offset_map.get(best_step_off)
            if sidx is not None:
                step_idx = sidx

        loops.append(
            _LoopRegion(
                header_idx=header_idx,
                exit_idx=exit_idx,
                step_idx=step_idx,
                back_jump_idx=i,
            )
        )

    return loops


def _find_enclosing_loop(loops: list[_LoopRegion], pc: int) -> _LoopRegion | None:
    """Find the innermost loop containing *pc*."""
    best: _LoopRegion | None = None
    for loop in loops:
        if loop.header_idx <= pc <= loop.back_jump_idx:
            if best is None or (loop.back_jump_idx - loop.header_idx) < (
                best.back_jump_idx - best.header_idx
            ):
                best = loop
    return best


# Tcl numeric helpers


def _resolve_var_name(name: str) -> str:
    """Strip ``${...}`` wrapping from a variable name in the LVT.

    The codegen encodes ``$x`` arguments as ``${x}`` in the local variable
    table.  At execution time we need the bare name so ``get_var`` /
    ``set_var`` can find the right slot.
    """
    if name.startswith("${") and name.endswith("}"):
        return name[2:-1]
    if name.startswith("$"):
        return name[1:]
    return name


def _resolve_dynamic_index(name: str, interp: object) -> str:
    """Substitute ``$var`` / ``[cmd]`` inside an array element index.

    For example, ``testConstraints($constraint)`` becomes
    ``testConstraints(singleTestInterp)`` when ``$constraint`` is
    ``singleTestInterp``.
    """
    paren = name.find("(")
    if paren == -1 or not name.endswith(")"):
        return name
    elem_expr = name[paren + 1 : -1]
    if "$" not in elem_expr and "[" not in elem_expr:
        return name
    from .substitution import substitute

    elem_val = substitute(elem_expr, interp)  # type: ignore[arg-type]
    return name[:paren] + "(" + elem_val + ")"


def _parse_int(s: str) -> int:
    """Parse *s* as an integer, accepting Tcl's 0x/0o/0b prefixes."""
    orig = s
    s = s.strip()
    try:
        return int(s, 0)
    except (ValueError, TypeError):
        pass
    # Fall back to base-10 — handles +07 and bare leading-zero literals
    # that Python's int(s, 0) rejects as ambiguous.
    try:
        return int(s, 10)
    except (ValueError, TypeError):
        pass
    # Handle boolean literals
    low = s.lower()
    if low in ("true", "yes", "on"):
        return 1
    if low in ("false", "no", "off"):
        return 0
    # Tcl reports "expected integer but got a list" when the value
    # looks like a multi-element list (contains unquoted whitespace).
    stripped = orig.strip()
    if " " in stripped or "\t" in stripped:
        raise TclError("expected integer but got a list")
    raise TclError(f'expected integer but got "{orig}"')


def _parse_int_strict(s: str) -> int:
    """Parse *s* as an integer WITHOUT accepting boolean literals.

    Tcl's ``incr`` command does not accept boolean strings like
    ``yes``/``no``/``true``/``false`` — only numeric integer literals.
    """
    orig = s
    s = s.strip()
    try:
        return int(s, 0)
    except (ValueError, TypeError):
        pass
    try:
        return int(s, 10)
    except (ValueError, TypeError):
        pass
    raise TclError(f'expected integer but got "{orig}"')


def _parse_tcl_index(s: str, list_len: int) -> int:
    """Parse a Tcl index expression like ``end``, ``end-1``, or a plain int."""
    s = s.strip()
    if s == "end":
        return list_len - 1
    if s.startswith("end-"):
        try:
            offset = int(s[4:])
            return list_len - 1 - offset
        except ValueError:
            pass
    if s.startswith("end+"):
        try:
            offset = int(s[4:])
            return list_len - 1 + offset
        except ValueError:
            pass
    return _parse_int(s)


# Operator symbol map for error messages

_OP_SYMBOLS: dict[str, str] = {
    "add": "+",
    "sub": "-",
    "mult": "*",
    "div": "/",
    "mod": "%",
    "expon": "**",
    "lshift": "<<",
    "rshift": ">>",
    "bitor": "|",
    "bitxor": "^",
    "bitand": "&",
    "bitnot": "~",
    "uminus": "-",
    "uplus": "+",
}


def _parse_number(s: str) -> int | float:
    """Parse *s* as int or float (Tcl semantics: try int first)."""
    s = s.strip()
    low = s.lower()
    if low in ("true", "yes", "on"):
        return 1
    if low in ("false", "no", "off"):
        return 0
    try:
        return int(s, 0)
    except (ValueError, TypeError):
        pass
    # Tcl 9.0 treats leading-zero decimals (e.g. "0005") as integers.
    # Python's int(x, 0) rejects them, so try base-10 explicitly.
    try:
        return int(s, 10)
    except (ValueError, TypeError):
        pass
    try:
        return float(s)
    except (ValueError, TypeError):
        raise TclError(f'expected number but got "{s}"') from None


_BOOLEAN_STRINGS = frozenset({"true", "false", "yes", "no", "on", "off"})


def _parse_number_arith(s: str, op_name: str, *, position: str = "left") -> int | float:
    """Parse *s* as a numeric value for an arithmetic operator.

    Tcl 9.0 rejects boolean string literals (``yes``/``no``/``true``/
    ``false``/``on``/``off``) in arithmetic context with *"cannot use
    non-numeric string … as … operand of …"*.
    """
    stripped = s.strip()
    low = stripped.lower()
    op_sym = _OP_SYMBOLS.get(op_name, op_name)
    if low in _BOOLEAN_STRINGS:
        raise TclError(f'cannot use non-numeric string "{s}" as {position} operand of "{op_sym}"')
    try:
        return int(stripped, 0)
    except (ValueError, TypeError):
        pass
    try:
        return int(stripped, 10)
    except (ValueError, TypeError):
        pass
    try:
        return float(stripped)
    except (ValueError, TypeError):
        raise TclError(
            f'cannot use non-numeric string "{s}" as {position} operand of "{op_sym}"'
        ) from None


def _parse_int_arith(s: str, op_name: str, *, position: str = "left") -> int:
    """Parse *s* as an integer for a bitwise/integer operator.

    Rejects boolean strings and floating-point values with Tcl 9.0
    error messages.
    """
    stripped = s.strip()
    low = stripped.lower()
    op_sym = _OP_SYMBOLS.get(op_name, op_name)
    if low in _BOOLEAN_STRINGS:
        raise TclError(f'cannot use non-numeric string "{s}" as {position} operand of "{op_sym}"')
    try:
        return int(stripped, 0)
    except (ValueError, TypeError):
        pass
    try:
        return int(stripped, 10)
    except (ValueError, TypeError):
        pass
    # Check for float — bitwise ops reject floats
    try:
        float(stripped)
        raise TclError(f'cannot use floating-point value "{s}" as {position} operand of "{op_sym}"')
    except ValueError:
        pass
    raise TclError(f'cannot use non-numeric string "{s}" as {position} operand of "{op_sym}"')


def _parse_number_unary(s: str, op_name: str) -> int | float:
    """Parse *s* for a unary arithmetic operator (rejects boolean strings)."""
    stripped = s.strip()
    low = stripped.lower()
    op_sym = _OP_SYMBOLS.get(op_name, op_name)
    if low in _BOOLEAN_STRINGS:
        raise TclError(f'cannot use non-numeric string "{s}" as operand of "{op_sym}"')
    try:
        return int(stripped, 0)
    except (ValueError, TypeError):
        pass
    try:
        return int(stripped, 10)
    except (ValueError, TypeError):
        pass
    try:
        return float(stripped)
    except (ValueError, TypeError):
        raise TclError(f'cannot use non-numeric string "{s}" as operand of "{op_sym}"') from None


def _parse_int_unary(s: str, op_name: str) -> int:
    """Parse *s* as integer for a unary bitwise operator (rejects booleans and floats)."""
    stripped = s.strip()
    low = stripped.lower()
    op_sym = _OP_SYMBOLS.get(op_name, op_name)
    if low in _BOOLEAN_STRINGS:
        raise TclError(f'cannot use non-numeric string "{s}" as operand of "{op_sym}"')
    try:
        return int(stripped, 0)
    except (ValueError, TypeError):
        pass
    try:
        return int(stripped, 10)
    except (ValueError, TypeError):
        pass
    try:
        float(stripped)
        raise TclError(f'cannot use floating-point value "{s}" as operand of "{op_sym}"')
    except ValueError:
        pass
    raise TclError(f'cannot use non-numeric string "{s}" as operand of "{op_sym}"')


def _parse_boolean(s: str) -> bool:
    """Parse a Tcl boolean value."""
    s = s.strip().lower()
    if s in ("1", "true", "yes", "on"):
        return True
    if s in ("0", "false", "no", "off"):
        return False
    try:
        return int(s, 0) != 0
    except (ValueError, TypeError):
        pass
    try:
        return float(s) != 0
    except (ValueError, TypeError):
        raise TclError(f'expected boolean value but got "{s}"') from None


def _format_number(v: int | float) -> str:
    """Format a number back to a Tcl string.

    Uses Python's ``repr()`` which produces the shortest round-trip
    representation — the same algorithm Tcl 9.0 uses.  Special values
    use Tcl capitalisation: ``Inf``, ``-Inf``, ``NaN``.
    """
    if isinstance(v, float):
        if math.isinf(v):
            return "-Inf" if v < 0 else "Inf"
        if math.isnan(v):
            return "NaN"
        return repr(v)
    try:
        return str(v)
    except ValueError:
        # Python raises ValueError when an integer exceeds the digit
        # limit for string conversion (default 4300/10000 digits).
        # Tcl has no such limit, but for safety convert via hex.
        sign = "-" if v < 0 else ""
        return sign + hex(abs(v))


# Arithmetic helpers (Tcl 9.0 semantics)


def _arith_binary(a_str: str, b_str: str, op_name: str) -> str:
    """Perform a binary arithmetic operation with Tcl type rules."""
    a = _parse_number_arith(a_str, op_name, position="left")
    b = _parse_number_arith(b_str, op_name, position="right")

    a_is_float = isinstance(a, float)
    b_is_float = isinstance(b, float)

    # Promote to float if either operand is float.
    if a_is_float or b_is_float:
        a, b = float(a), float(b)

    match op_name:
        case "add":
            result = a + b
        case "sub":
            result = a - b
        case "mult":
            result = a * b
        case "div":
            if b == 0:
                # IEEE platforms: float/0.0 → Inf/-Inf/NaN
                if isinstance(a, float) or isinstance(b, float):
                    fa, fb = float(a), float(b)
                    result = math.copysign(math.inf, fa) if fa != 0.0 else math.nan
                else:
                    raise TclError("divide by zero", error_code="ARITH DIVZERO {divide by zero}")
            elif isinstance(a, int) and isinstance(b, int):
                # Tcl uses floor division for integers
                result = a // b
            else:
                result = a / b
        case "mod":
            # Tcl 9.0 requires integer operands for %
            if b_is_float:
                raise TclError(f'cannot use floating-point value "{b_str}" as right operand of "%"')
            if a_is_float:
                raise TclError(f'cannot use floating-point value "{a_str}" as left operand of "%"')
            if b == 0:
                raise TclError("divide by zero", error_code="ARITH DIVZERO {divide by zero}")
            result = a % b
        case "expon":
            if isinstance(a, int) and isinstance(b, int):
                if b < 0:
                    if a == 0:
                        raise TclError("exponentiation of zero by negative power")
                    try:
                        fa = float(a)
                    except OverflowError:
                        # Base too large for float → reciprocal underflows to 0
                        result = 0.0
                    else:
                        try:
                            result = fa**b
                        except OverflowError:
                            result = math.copysign(math.inf, a) if a < 0 and b % 2 else math.inf
                else:
                    try:
                        result = a**b
                    except (MemoryError, OverflowError):
                        raise TclError("integer value too large to represent") from None
            else:
                fa, fb = float(a), float(b)
                if fa == 0.0 and fb < 0:
                    raise TclError("exponentiation of zero by negative power")
                # Tcl 9.0: negative base with non-integer exponent is a domain error
                if fa < 0 and (not math.isfinite(fb) or not fb.is_integer()):
                    raise TclError(
                        "domain error: argument not in valid range",
                        error_code="ARITH DOMAIN {domain error: argument not in valid range}",
                    )
                try:
                    result = fa**fb
                except OverflowError:
                    result = math.copysign(math.inf, fa) if fa < 0 and int(fb) % 2 else math.inf
        case _:
            raise TclError(f"unknown arithmetic op: {op_name}")

    return _format_number(result)


def _bitwise_binary(a_str: str, b_str: str, op_name: str) -> str:
    """Perform a bitwise operation (requires integer operands)."""
    a = _parse_int_arith(a_str, op_name, position="left")
    b = _parse_int_arith(b_str, op_name, position="right")
    try:
        match op_name:
            case "lshift":
                if b < 0:
                    raise TclError("negative shift argument")
                if a != 0 and b > 131072:
                    raise TclError("integer value too large to represent")
                result = a << b
            case "rshift":
                if b < 0:
                    raise TclError("negative shift argument")
                result = a >> b
            case "bitor":
                result = a | b
            case "bitxor":
                result = a ^ b
            case "bitand":
                result = a & b
            case _:
                raise TclError(f"unknown bitwise op: {op_name}")
    except MemoryError:
        raise TclError("integer value too large to represent") from None
    try:
        return str(result)
    except ValueError:
        # Integer exceeds Python's digit limit for string conversion.
        sign = "-" if result < 0 else ""
        return sign + hex(abs(result))


def _compare(a_str: str, b_str: str, op_name: str) -> str:
    """Numeric comparison with string fallback; returns "0" or "1".

    In Tcl, the relational operators (``<``, ``>``, ``<=``, ``>=``,
    ``==``, ``!=``) first try numeric comparison and fall back to
    string comparison when either operand is not a valid number.
    """
    try:
        a_num = _parse_number(a_str)
        b_num = _parse_number(b_str)
        match op_name:
            case "eq":
                return "1" if a_num == b_num else "0"
            case "neq":
                return "1" if a_num != b_num else "0"
            case "lt":
                return "1" if a_num < b_num else "0"
            case "gt":
                return "1" if a_num > b_num else "0"
            case "le":
                return "1" if a_num <= b_num else "0"
            case "ge":
                return "1" if a_num >= b_num else "0"
            case _:
                raise TclError(f"unknown compare op: {op_name}")
    except TclError:
        match op_name:
            case "eq":
                return "1" if a_str == b_str else "0"
            case "neq":
                return "1" if a_str != b_str else "0"
            case "lt":
                return "1" if a_str < b_str else "0"
            case "gt":
                return "1" if a_str > b_str else "0"
            case "le":
                return "1" if a_str <= b_str else "0"
            case "ge":
                return "1" if a_str >= b_str else "0"
            case _:
                raise TclError(f"unknown compare op: {op_name}")


def _str_compare(a_str: str, b_str: str, op_name: str) -> str:
    """String comparison; returns "0"/"1" or int for strcmp."""
    match op_name:
        case "streq":
            return "1" if a_str == b_str else "0"
        case "strneq":
            return "1" if a_str != b_str else "0"
        case "strcmp":
            if a_str < b_str:
                return "-1"
            if a_str > b_str:
                return "1"
            return "0"
        case "strlt":
            return "1" if a_str < b_str else "0"
        case "strgt":
            return "1" if a_str > b_str else "0"
        case "strle":
            return "1" if a_str <= b_str else "0"
        case "strge":
            return "1" if a_str >= b_str else "0"
        case _:
            raise TclError(f"unknown string compare: {op_name}")


# Jump offset resolution


def _build_offset_map(instrs: list[Instruction]) -> dict[int, int]:
    """Map byte offsets to instruction indices for jump resolution."""
    result: dict[int, int] = {}
    for i, instr in enumerate(instrs):
        result[instr.offset] = i
    return result


# Bytecode VM

# Sentinel marker used to delimit {*} expanded call arguments on the stack.
_EXPAND_SENTINEL = "\0__expand_sentinel__\0"

# Matches ``::tcl::<base>::<sub>`` — used by INVOKE_REPLACE to resolve
# fully-qualified ensemble command names back to base + subcommand.
_FQ_CMD_RE = re.compile(r"^::tcl::(\w+)::(\w+)$")


class BytecodeVM:
    """Stack-based bytecode interpreter."""

    def __init__(
        self,
        interp: TclInterp,
        *,
        debug_hook: object | None = None,
    ) -> None:
        self.interp = interp
        self._debug_hook = debug_hook

    def execute(self, asm: FunctionAsm) -> TclResult:
        """Execute a ``FunctionAsm`` and return the result."""
        stack: list[str] = []
        last_result: str = ""  # track last popped value for DONE
        pc = 0
        instrs = asm.instructions
        literals = asm.literals.entries()
        lvt_names = asm.lvt.entries()
        frame = self.interp.current_frame
        # Cache offset_map and loops on the asm object to avoid
        # recomputing on every execute() call for the same bytecode.
        offset_map = asm._cached_offset_map
        if offset_map is None:
            offset_map = _build_offset_map(instrs)
            asm._cached_offset_map = offset_map
        loops = asm._cached_loops
        if loops is None:
            loops = _find_loops(instrs, offset_map, asm.labels)
            asm._cached_loops = loops

        debug_hook = self._debug_hook
        _prev_source_line = -1

        while pc < len(instrs):
            instr = instrs[pc]

            # Debug hook: fire at source line boundaries
            if debug_hook is not None and instr.source_line != _prev_source_line:
                _prev_source_line = instr.source_line
                if instr.source_line > 0:
                    from debugger.types import DebugAction

                    action = debug_hook(instr, pc, stack, frame)
                    if action == DebugAction.STOP:
                        return TclResult(value=last_result)

            op = instr.op

            match op:
                # Stack operations
                case Op.PUSH1 | Op.PUSH4:
                    idx = instr.operands[0]
                    if isinstance(idx, int):
                        value = literals[idx]
                        # Braced values suppress all substitution.
                        # _process_literals() marks STR tokens with sentinel
                        # markers; other braced literals (proc bodies etc.)
                        # use raw {…} and are matched by _braces_balanced.
                        # Raw-prefixed literals (interpolation template parts)
                        # are pushed exactly as-is.
                        if value.startswith(_RAW_PREFIX):
                            value = value[len(_RAW_PREFIX) :]
                        elif value.startswith(_BRACE_OPEN) and value.endswith(_BRACE_CLOSE):
                            value = value[len(_BRACE_OPEN) : -len(_BRACE_CLOSE)]
                        elif (
                            value.startswith("{")
                            and value.endswith("}")
                            and _braces_balanced(value)
                        ):
                            value = value[1:-1]
                        elif "$" in value or ("[" in value and "]" in value):
                            from .substitution import subst_command

                            value = subst_command(value, self.interp)
                        stack.append(value)
                    else:
                        stack.append("")

                case Op.POP:
                    if stack:
                        last_result = stack.pop()

                case Op.DUP:
                    if stack:
                        stack.append(stack[-1])
                    else:
                        stack.append("")

                # Variable operations
                case Op.LOAD_SCALAR1 | Op.LOAD_SCALAR4:
                    slot = instr.operands[0]
                    if isinstance(slot, int):
                        name = _resolve_var_name(lvt_names[slot])
                    else:
                        name = str(slot)
                    name = _resolve_dynamic_index(name, self.interp)
                    stack.append(frame.get_var(name))

                case Op.STORE_SCALAR1 | Op.STORE_SCALAR4:
                    slot = instr.operands[0]
                    if isinstance(slot, int):
                        name = _resolve_var_name(lvt_names[slot])
                    else:
                        name = str(slot)
                    name = _resolve_dynamic_index(name, self.interp)
                    value = stack[-1] if stack else ""
                    frame.set_var(name, value)

                case Op.INCR_SCALAR1:
                    slot = instr.operands[0]
                    if isinstance(slot, int):
                        name = _resolve_var_name(lvt_names[slot])
                    else:
                        name = str(slot)
                    amount = _parse_int_strict(stack.pop()) if stack else 1
                    old = _parse_int_strict(frame.get_var(name, default="0"))
                    result_str = str(old + amount)
                    frame.set_var(name, result_str)
                    stack.append(result_str)

                case Op.INCR_SCALAR1_IMM:
                    slot = instr.operands[0]
                    if isinstance(slot, int):
                        name = _resolve_var_name(lvt_names[slot])
                    else:
                        name = str(slot)
                    amount = instr.operands[1] if len(instr.operands) > 1 else 1
                    if isinstance(amount, str):
                        amount = int(amount)
                    old = _parse_int_strict(frame.get_var(name, default="0"))
                    result_str = str(old + amount)
                    frame.set_var(name, result_str)
                    stack.append(result_str)

                case Op.APPEND_SCALAR1:
                    slot = instr.operands[0]
                    if isinstance(slot, int):
                        name = _resolve_var_name(lvt_names[slot])
                    else:
                        name = str(slot)
                    value = stack.pop() if stack else ""
                    old = frame.get_var(name, default="")
                    new_val = old + value
                    frame.set_var(name, new_val)
                    stack.append(new_val)

                case Op.LAPPEND_SCALAR1:
                    slot = instr.operands[0]
                    if isinstance(slot, int):
                        name = _resolve_var_name(lvt_names[slot])
                    else:
                        name = str(slot)
                    value = stack.pop() if stack else ""
                    old = frame.get_var(name, default="")
                    new_val = (old + " " + value) if old else value
                    frame.set_var(name, new_val)
                    stack.append(new_val)

                # Stack-based variable operations (top-level)
                case Op.STORE_STK:
                    # TOS = value, TOS-1 = variable name
                    value = stack.pop() if stack else ""
                    name = _resolve_var_name(stack.pop()) if stack else ""
                    frame.set_var(name, value)
                    stack.append(value)

                case Op.LOAD_STK:
                    # TOS = variable name
                    name = _resolve_var_name(stack.pop()) if stack else ""
                    stack.append(frame.get_var(name))

                case Op.INCR_STK_IMM:
                    # TOS = variable name, operand = immediate amount
                    name = _resolve_var_name(stack.pop()) if stack else ""
                    amount = instr.operands[0] if instr.operands else 1
                    if isinstance(amount, str):
                        amount = int(amount)
                    old = _parse_int_strict(frame.get_var(name, default="0"))
                    result_str = str(old + amount)
                    frame.set_var(name, result_str)
                    stack.append(result_str)

                case Op.APPEND_STK:
                    # TOS = value, TOS-1 = variable name
                    value = stack.pop() if stack else ""
                    name = _resolve_var_name(stack.pop()) if stack else ""
                    old = frame.get_var(name, default="")
                    new_val = old + value
                    frame.set_var(name, new_val)
                    stack.append(new_val)

                case Op.LAPPEND_STK:
                    # TOS = value, TOS-1 = variable name
                    value = stack.pop() if stack else ""
                    name = _resolve_var_name(stack.pop()) if stack else ""
                    old = frame.get_var(name, default="")
                    new_val = (old + " " + value) if old else value
                    frame.set_var(name, new_val)
                    stack.append(new_val)

                case Op.LAPPEND_LIST_STK:
                    # TOS = list value, TOS-1 = variable name
                    list_val = stack.pop() if stack else ""
                    name = _resolve_var_name(stack.pop()) if stack else ""
                    old = frame.get_var(name, default="")
                    new_val = (old + " " + list_val) if old else list_val
                    frame.set_var(name, new_val)
                    stack.append(new_val)

                case Op.INCR_STK:
                    # TOS = increment amount, TOS-1 = variable name
                    amount = _parse_int_strict(stack.pop()) if stack else 1
                    name = _resolve_var_name(stack.pop()) if stack else ""
                    old = _parse_int_strict(frame.get_var(name, default="0"))
                    result_str = str(old + amount)
                    frame.set_var(name, result_str)
                    stack.append(result_str)

                case Op.UNSET_STK:
                    # TOS = variable name, operand = flags (0=nocomplain, 1=complain)
                    name = _resolve_var_name(stack.pop()) if stack else ""
                    flags = instr.operands[0] if instr.operands else 1
                    nocomplain = flags == 0
                    frame.unset_var(name, nocomplain=nocomplain)

                # Stack-based array variable operations
                case Op.STORE_ARRAY_STK:
                    # TOS = value, TOS-1 = element, TOS-2 = array name
                    value = stack.pop() if stack else ""
                    elem = stack.pop() if stack else ""
                    arr_name = stack.pop() if stack else ""
                    frame.set_var(f"{arr_name}({elem})", value)
                    stack.append(value)

                case Op.LOAD_ARRAY_STK:
                    # TOS = element, TOS-1 = array name
                    elem = stack.pop() if stack else ""
                    arr_name = stack.pop() if stack else ""
                    stack.append(frame.get_var(f"{arr_name}({elem})"))

                case Op.INCR_ARRAY_STK_IMM:
                    # TOS = element, TOS-1 = array name, operand = immediate amount
                    elem = stack.pop() if stack else ""
                    arr_name = stack.pop() if stack else ""
                    amount = instr.operands[0] if instr.operands else 1
                    if isinstance(amount, str):
                        amount = int(amount)
                    name = f"{arr_name}({elem})"
                    old = _parse_int_strict(frame.get_var(name, default="0"))
                    result_str = str(old + amount)
                    frame.set_var(name, result_str)
                    stack.append(result_str)

                case Op.LAPPEND_LIST_ARRAY_STK:
                    # TOS = list value, TOS-1 = element, TOS-2 = array name
                    list_val = stack.pop() if stack else ""
                    elem = stack.pop() if stack else ""
                    arr_name = stack.pop() if stack else ""
                    name = f"{arr_name}({elem})"
                    old = frame.get_var(name, default="")
                    new_val = (old + " " + list_val) if old else list_val
                    frame.set_var(name, new_val)
                    stack.append(new_val)

                # Dict specialised opcodes
                case Op.DICT_GET:
                    num_keys = instr.operands[0]
                    if isinstance(num_keys, str):
                        num_keys = int(num_keys)
                    keys = [stack.pop() for _ in range(num_keys)]
                    keys.reverse()
                    dict_val = stack.pop() if stack else ""
                    d = _dict_from_list(dict_val)
                    if num_keys == 0:
                        stack.append(dict_val)
                    else:
                        stack.append(_nested_get(d, keys))

                case Op.DICT_EXISTS:
                    num_keys = instr.operands[0]
                    if isinstance(num_keys, str):
                        num_keys = int(num_keys)
                    keys = [stack.pop() for _ in range(num_keys)]
                    keys.reverse()
                    dict_val = stack.pop() if stack else ""
                    try:
                        d = _dict_from_list(dict_val)
                        stack.append("1" if _nested_exists(d, keys) else "0")
                    except TclError:
                        stack.append("0")

                case Op.VERIFY_DICT:
                    # Verify TOS is a valid dict; leave on stack
                    val = stack[-1] if stack else ""
                    _dict_from_list(val)  # raises TclError if invalid

                case Op.INVOKE_REPLACE:
                    self.interp._error_line = instr.source_line
                    # invokeReplace argc replace_count
                    # Stack: arg0 arg1 ... argN-1 replacementCmd
                    argc = instr.operands[0]
                    if isinstance(argc, str):
                        argc = int(argc)
                    replace_count = instr.operands[1] if len(instr.operands) > 1 else 2
                    if isinstance(replace_count, str):
                        replace_count = int(replace_count)
                    replacement_cmd = stack.pop()
                    orig_args = stack[-(argc):]
                    del stack[-(argc):]
                    # Strip the replaced prefix words and resolve the FQ
                    # command name to a base command + subcommand.
                    # e.g. "::tcl::dict::lappend" → invoke("dict", ["lappend", ...])
                    remaining = list(orig_args[replace_count:])
                    _m = _FQ_CMD_RE.match(replacement_cmd)
                    if _m:
                        base_cmd = _m.group(1)
                        sub_cmd = _m.group(2)
                        result = self.interp.invoke(base_cmd, [sub_cmd] + remaining)
                    else:
                        result = self.interp.invoke(replacement_cmd, remaining)
                    stack.append(result.value)

                case Op.EXIST_STK:
                    # existStk: pop var name, push "1" if it exists, "0" otherwise
                    var_name = stack.pop() if stack else ""
                    exists = self.interp.current_frame.exists(var_name)
                    stack.append("1" if exists else "0")

                # {*} expansion
                case Op.EXPAND_START:
                    # Push a sentinel to mark the start of an expanded call
                    stack.append(_EXPAND_SENTINEL)

                case Op.EXPAND_STKTOP:
                    # Pop top value, split as Tcl list, push all elements
                    val = stack.pop() if stack else ""
                    elements = _split_list(val)
                    stack.extend(elements)

                case Op.INVOKE_EXPANDED:
                    self.interp._error_line = instr.source_line
                    self.interp._error_cmd_text = instr.source_cmd_text
                    # Pop all args since the EXPAND_START sentinel
                    args: list[str] = []
                    while stack and stack[-1] != _EXPAND_SENTINEL:
                        args.append(stack.pop())
                    if stack:
                        stack.pop()  # remove sentinel
                    args.reverse()
                    if not args:
                        # All expansions produced nothing — no-op
                        stack.append("")
                    else:
                        cmd_name = args[0]
                        cmd_args = args[1:]
                        try:
                            result = self.interp.invoke(cmd_name, cmd_args)
                            stack.append(result.value)
                        except TclBreak:
                            loop = _find_enclosing_loop(loops, pc)
                            if loop is not None:
                                pc = loop.exit_idx
                                continue
                            raise
                        except TclContinue:
                            loop = _find_enclosing_loop(loops, pc)
                            if loop is not None:
                                pc = loop.step_idx
                                continue
                            raise
                        except TclReturn:
                            raise
                        except TclError:
                            raise

                # Command dispatch
                case Op.INVOKE_STK1 | Op.INVOKE_STK4:
                    self.interp._error_line = instr.source_line
                    self.interp._error_cmd_text = instr.source_cmd_text
                    argc = instr.operands[0]
                    if isinstance(argc, str):
                        argc = int(argc)
                    args = stack[-argc:]
                    del stack[-argc:]
                    cmd_name = args[0] if args else ""
                    cmd_args = args[1:]
                    try:
                        result = self.interp.invoke(cmd_name, cmd_args)
                        stack.append(result.value)
                    except TclBreak:
                        loop = _find_enclosing_loop(loops, pc)
                        if loop is not None:
                            pc = loop.exit_idx
                            continue
                        raise
                    except TclContinue:
                        loop = _find_enclosing_loop(loops, pc)
                        if loop is not None:
                            pc = loop.step_idx
                            continue
                        raise
                    except TclReturn:
                        raise
                    except TclError:
                        raise

                case Op.EVAL_STK:
                    script = stack.pop() if stack else ""
                    try:
                        result = self.interp.eval(script)
                        stack.append(result.value)
                    except TclBreak:
                        loop = _find_enclosing_loop(loops, pc)
                        if loop is not None:
                            pc = loop.exit_idx
                            continue
                        raise
                    except TclContinue:
                        loop = _find_enclosing_loop(loops, pc)
                        if loop is not None:
                            pc = loop.step_idx
                            continue
                        raise

                case Op.EXPR_STK:
                    expr_str = stack.pop() if stack else ""
                    result_str = self.interp.eval_expr(expr_str)
                    stack.append(result_str)

                # Arithmetic
                case Op.ADD:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_arith_binary(a, b, "add"))
                case Op.SUB:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_arith_binary(a, b, "sub"))
                case Op.MULT:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_arith_binary(a, b, "mult"))
                case Op.DIV:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_arith_binary(a, b, "div"))
                case Op.MOD:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_arith_binary(a, b, "mod"))
                case Op.EXPON:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_arith_binary(a, b, "expon"))

                # Bitwise
                case Op.LSHIFT:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_bitwise_binary(a, b, "lshift"))
                case Op.RSHIFT:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_bitwise_binary(a, b, "rshift"))
                case Op.BITOR:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_bitwise_binary(a, b, "bitor"))
                case Op.BITXOR:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_bitwise_binary(a, b, "bitxor"))
                case Op.BITAND:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_bitwise_binary(a, b, "bitand"))

                # Numeric comparison
                case Op.EQ:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_compare(a, b, "eq"))
                case Op.NEQ:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_compare(a, b, "neq"))
                case Op.LT:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_compare(a, b, "lt"))
                case Op.GT:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_compare(a, b, "gt"))
                case Op.LE:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_compare(a, b, "le"))
                case Op.GE:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_compare(a, b, "ge"))

                # String comparison
                case Op.STR_EQ:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_str_compare(a, b, "streq"))
                case Op.STR_NEQ:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_str_compare(a, b, "strneq"))
                case Op.STR_CMP:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_str_compare(a, b, "strcmp"))
                case Op.STR_LT:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_str_compare(a, b, "strlt"))
                case Op.STR_GT:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_str_compare(a, b, "strgt"))
                case Op.STR_LE:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_str_compare(a, b, "strle"))
                case Op.STR_GE:
                    b, a = stack.pop(), stack.pop()
                    stack.append(_str_compare(a, b, "strge"))

                # Unary
                case Op.UMINUS:
                    a = _parse_number_unary(stack.pop(), "uminus")
                    stack.append(_format_number(-a))
                case Op.UPLUS:
                    a = _parse_number_unary(stack.pop(), "uplus")
                    stack.append(_format_number(a))
                case Op.BITNOT:
                    a = _parse_int_unary(stack.pop(), "bitnot")
                    stack.append(str(~a))
                case Op.LNOT:
                    a = _parse_boolean(stack.pop())
                    stack.append("0" if a else "1")

                # Logical
                case Op.LAND:
                    b = _parse_boolean(stack.pop())
                    a = _parse_boolean(stack.pop())
                    stack.append("1" if (a and b) else "0")
                case Op.LOR:
                    b = _parse_boolean(stack.pop())
                    a = _parse_boolean(stack.pop())
                    stack.append("1" if (a or b) else "0")

                # String operations
                case Op.STR_CONCAT1:
                    count = instr.operands[0]
                    if isinstance(count, str):
                        count = int(count)
                    parts = stack[-count:]
                    del stack[-count:]
                    stack.append("".join(parts))

                case Op.STR_LEN:
                    s = stack.pop()
                    stack.append(str(len(s)))

                case Op.STR_INDEX:
                    idx_str = stack.pop()
                    s = stack.pop()
                    idx = _parse_int(idx_str)
                    if 0 <= idx < len(s):
                        stack.append(s[idx])
                    else:
                        stack.append("")

                case Op.STR_MAP:
                    s = stack.pop()
                    mapping_str = stack.pop()
                    mapping_list = _split_list(mapping_str)
                    if len(mapping_list) % 2 != 0:
                        raise TclError("char map list unbalanced")
                    pairs = [
                        (mapping_list[j], mapping_list[j + 1])
                        for j in range(0, len(mapping_list), 2)
                    ]
                    result_parts: list[str] = []
                    i_map = 0
                    while i_map < len(s):
                        matched = False
                        for key, val in pairs:
                            if key and s[i_map:].startswith(key):
                                result_parts.append(val)
                                i_map += len(key)
                                matched = True
                                break
                        if not matched:
                            result_parts.append(s[i_map])
                            i_map += 1
                    stack.append("".join(result_parts))

                case Op.STR_FIND:
                    s = stack.pop()
                    needle = stack.pop()
                    stack.append(str(s.find(needle)))

                case Op.STR_RFIND:
                    s = stack.pop()
                    needle = stack.pop()
                    stack.append(str(s.rfind(needle)))

                case Op.STR_REPLACE:
                    # string replace string first last ?newString?
                    new = stack.pop()
                    s = stack.pop()
                    stack.append(new)  # TODO: proper string replace

                case Op.STR_TRIM:
                    chars = stack.pop() if stack else None
                    s = stack.pop() if stack else ""
                    stack.append(s.strip(chars) if chars else s.strip())

                case Op.STR_TRIM_LEFT:
                    chars = stack.pop() if stack else None
                    s = stack.pop() if stack else ""
                    stack.append(s.lstrip(chars) if chars else s.lstrip())

                case Op.STR_TRIM_RIGHT:
                    chars = stack.pop() if stack else None
                    s = stack.pop() if stack else ""
                    stack.append(s.rstrip(chars) if chars else s.rstrip())

                case Op.STR_MATCH:
                    string = stack.pop()
                    pattern = stack.pop()
                    nocase = instr.operands[0] if instr.operands else 0
                    if isinstance(nocase, str):
                        nocase = int(nocase)
                    if nocase:
                        import fnmatch

                        result = fnmatch.fnmatchcase(string.lower(), pattern.lower())
                    else:
                        import fnmatch

                        result = fnmatch.fnmatchcase(string, pattern)
                    stack.append("1" if result else "0")

                case Op.STR_UPPER:
                    s = stack.pop()
                    stack.append(s.upper())

                case Op.STR_LOWER:
                    s = stack.pop()
                    stack.append(s.lower())

                case Op.STR_TITLE:
                    s = stack.pop()
                    stack.append(s.title())

                case Op.STR_RANGE:
                    last_str = stack.pop()
                    first_str = stack.pop()
                    s = stack.pop()
                    first = _parse_tcl_index(first_str, len(s))
                    last = _parse_tcl_index(last_str, len(s))
                    if first < 0:
                        first = 0
                    if last >= len(s):
                        last = len(s) - 1
                    if first > last:
                        stack.append("")
                    else:
                        stack.append(s[first : last + 1])

                case Op.STR_RANGE_IMM:
                    s = stack.pop()
                    first = instr.operands[0]
                    last = instr.operands[1]
                    if isinstance(first, str):
                        first = int(first)
                    if isinstance(last, str):
                        last = int(last)
                    slen = len(s)
                    if first <= _INDEX_END:
                        first = slen - 1 + (first - _INDEX_END)
                    if last <= _INDEX_END:
                        last = slen - 1 + (last - _INDEX_END)
                    if first < 0:
                        first = 0
                    if last >= slen:
                        last = slen - 1
                    if first > last:
                        stack.append("")
                    else:
                        stack.append(s[first : last + 1])

                case Op.STR_REVERSE:
                    s = stack.pop()
                    stack.append(s[::-1])

                case Op.STR_REPEAT:
                    count_str = stack.pop()
                    s = stack.pop()
                    count = _parse_int(count_str)
                    stack.append(s * max(0, count))

                case Op.REGEXP:
                    argc = instr.operands[0] if instr.operands else 3
                    if isinstance(argc, str):
                        argc = int(argc)
                    # Operand is encoded as argc + 1; actual stack items = argc - 1
                    actual_argc = argc - 1
                    args_list = stack[-actual_argc:]
                    del stack[-actual_argc:]
                    # Delegate to the full regexp command implementation
                    result = self.interp.invoke("regexp", args_list)
                    stack.append(result.value)

                # List operations
                case Op.LIST:
                    count = instr.operands[0]
                    if isinstance(count, str):
                        count = int(count)
                    elements = stack[-count:] if count > 0 else []
                    if count > 0:
                        del stack[-count:]
                    stack.append(" ".join(_list_escape(e) for e in elements))

                case Op.LIST_LENGTH:
                    lst = stack.pop()
                    stack.append(str(len(_split_list(lst))))

                case Op.LIST_INDEX:
                    idx_str = stack.pop()
                    lst = stack.pop()
                    elements = _split_list(lst)
                    idx = _parse_tcl_index(idx_str, len(elements))
                    if 0 <= idx < len(elements):
                        stack.append(elements[idx])
                    else:
                        stack.append("")

                case Op.LIST_INDEX_IMM:
                    lst = stack.pop()
                    elements = _split_list(lst)
                    idx = instr.operands[0]
                    if isinstance(idx, str):
                        idx = int(idx)
                    # _INDEX_END is used by compiler-generated code for "end"
                    if idx <= _INDEX_END:
                        idx = len(elements) - 1 + (idx - _INDEX_END)
                    if 0 <= idx < len(elements):
                        stack.append(elements[idx])
                    else:
                        stack.append("")

                case Op.LIST_RANGE_IMM:
                    lst = stack.pop()
                    elements = _split_list(lst)
                    first = instr.operands[0]
                    last = instr.operands[1]
                    if isinstance(first, str):
                        first = int(first)
                    if isinstance(last, str):
                        last = int(last)
                    n_elems = len(elements)
                    # _INDEX_END sentinel from compiler-generated code
                    if first <= _INDEX_END:
                        first = n_elems - 1 + (first - _INDEX_END)
                    if last <= _INDEX_END:
                        last = n_elems - 1 + (last - _INDEX_END)
                    if first < 0:
                        first = 0
                    if last >= n_elems:
                        last = n_elems - 1
                    if first > last or n_elems == 0:
                        stack.append("")
                    else:
                        selected = elements[first : last + 1]
                        stack.append(" ".join(_list_escape(e) for e in selected))

                case Op.LIST_IN:
                    lst = stack.pop()
                    val = stack.pop()
                    elements = _split_list(lst)
                    stack.append("1" if val in elements else "0")

                case Op.LIST_NOT_IN:
                    lst = stack.pop()
                    val = stack.pop()
                    elements = _split_list(lst)
                    stack.append("1" if val not in elements else "0")

                case Op.LIST_CONCAT:
                    b = stack.pop()
                    a = stack.pop()
                    a_elems = _split_list(a) if a else []
                    b_elems = _split_list(b) if b else []
                    combined = a_elems + b_elems
                    stack.append(" ".join(_list_escape(e) for e in combined) if combined else "")

                case Op.LREPLACE4:
                    argc = instr.operands[0]
                    if isinstance(argc, str):
                        argc = int(argc)
                    mode = instr.operands[1]
                    if isinstance(mode, str):
                        mode = int(mode)
                    # Pop argc values (pushed in order: list, idx args, elements)
                    vals = [stack.pop() for _ in range(argc)]
                    vals.reverse()
                    lst = _split_list(vals[0])
                    if mode == 1:
                        # lreplace: list first last ?elem ...?
                        first = _parse_lreplace_index(vals[1], len(lst))
                        last = _parse_lreplace_index(vals[2], len(lst))
                        new_elems = vals[3:]
                        first = max(0, first)
                        if last < first:
                            # last < first → insert before first, no deletion
                            result = lst[:first] + new_elems + lst[first:]
                        else:
                            last = min(len(lst) - 1, last)
                            result = lst[:first] + new_elems + lst[last + 1 :]
                    else:
                        # linsert: list index ?elem ...?
                        # In Tcl, "end" means after the last element for linsert,
                        # so parse with length+1 to map end→length.
                        idx = _parse_lreplace_index(vals[1], len(lst) + 1)
                        new_elems = vals[2:]
                        idx = max(0, min(idx, len(lst)))
                        result = lst[:idx] + new_elems + lst[idx:]
                    stack.append(" ".join(_list_escape(e) for e in result) if result else "")

                # Control flow
                case Op.JUMP1 | Op.JUMP4:
                    target = instr.operands[0]
                    if isinstance(target, str):
                        target_offset = asm.labels.get(target, 0)
                    else:
                        target_offset = target
                    target_idx = offset_map.get(target_offset)
                    if target_idx is not None:
                        pc = target_idx
                        continue
                    # If target not found, fall through

                case Op.JUMP_TRUE1 | Op.JUMP_TRUE4:
                    val = stack.pop() if stack else "0"
                    if _parse_boolean(val):
                        target = instr.operands[0]
                        if isinstance(target, str):
                            target_offset = asm.labels.get(target, 0)
                        else:
                            target_offset = target
                        target_idx = offset_map.get(target_offset)
                        if target_idx is not None:
                            pc = target_idx
                            continue

                case Op.JUMP_FALSE1 | Op.JUMP_FALSE4:
                    val = stack.pop() if stack else "0"
                    if not _parse_boolean(val):
                        target = instr.operands[0]
                        if isinstance(target, str):
                            target_offset = asm.labels.get(target, 0)
                        else:
                            target_offset = target
                        target_idx = offset_map.get(target_offset)
                        if target_idx is not None:
                            pc = target_idx
                            continue

                # Return / Done
                case Op.DONE:
                    return TclResult(
                        code=ReturnCode.OK,
                        value=stack[-1] if stack else last_result,
                    )

                case Op.RETURN_IMM:
                    self.interp._error_line = instr.source_line
                    value = stack[-1] if stack else last_result
                    ret_code = instr.operands[0] if instr.operands else 0
                    ret_level = instr.operands[1] if len(instr.operands) > 1 else 1
                    if isinstance(ret_code, str):
                        ret_code = int(ret_code)
                    if isinstance(ret_level, str):
                        ret_level = int(ret_level)
                    if ret_code == ReturnCode.ERROR and ret_level == 0:
                        # Compiled ``error`` command — extract errorInfo/errorCode
                        # from the options value on the stack.
                        opts_val = stack[-2] if len(stack) >= 2 else ""
                        ei_parts: list[str] = []
                        ec = "NONE"
                        ei_raw = ""
                        if opts_val:
                            opt_list = _split_list(opts_val)
                            j = 0
                            while j < len(opt_list) - 1:
                                if opt_list[j] == "-errorinfo" and opt_list[j + 1]:
                                    ei_raw = opt_list[j + 1]
                                    ei_parts = [ei_raw]
                                elif opt_list[j] == "-errorcode":
                                    ec = opt_list[j + 1]
                                j += 2
                        # Reconstruct the ``error`` command text for "while executing"
                        cmd_parts = ["error"]
                        if " " in value or "\t" in value or not value:
                            cmd_parts.append(f'"{value}"')
                        else:
                            cmd_parts.append(value)
                        if ei_raw:
                            cmd_parts.append(f'"{ei_raw}"' if " " in ei_raw else ei_raw)
                        if ec != "NONE":
                            cmd_parts.append(ec)
                        cmd_text = " ".join(cmd_parts)
                        if ei_parts:
                            # Seeded errorInfo — append "while executing" to it
                            ei_parts.append(f'    while executing\n"{cmd_text}"')
                        else:
                            ei_parts = [value, f'    while executing\n"{cmd_text}"']
                        raise TclError(value, error_code=ec, error_info=ei_parts)
                    raise TclReturn(value=value, code=ret_code, level=max(ret_level, 1))

                case Op.BREAK:
                    loop = _find_enclosing_loop(loops, pc)
                    if loop is not None:
                        pc = loop.exit_idx
                        continue
                    raise TclBreak()

                case Op.CONTINUE:
                    loop = _find_enclosing_loop(loops, pc)
                    if loop is not None:
                        pc = loop.step_idx
                        continue
                    raise TclContinue()

                # Exception handling
                # catch/try are desugared to barriers — these
                # opcodes are never emitted for VM execution.
                case Op.BEGIN_CATCH4:
                    pass

                case Op.END_CATCH:
                    pass

                case Op.PUSH_RESULT:
                    # Push the interpreter's current result
                    stack.append("")

                case Op.PUSH_RETURN_CODE:
                    stack.append("0")

                # Foreach / Switch
                # Desugared to barriers — never emitted for VM.
                case Op.FOREACH_START | Op.FOREACH_STEP | Op.FOREACH_END:
                    pass

                case Op.JUMP_TABLE:
                    value = stack.pop()
                    jt = instr.jump_table
                    if jt and value in jt:
                        target_label = jt[value]
                        target_off = asm.labels.get(target_label, -1)
                        target_idx = offset_map.get(target_off)
                        if target_idx is not None:
                            pc = target_idx
                            continue

                # Misc
                case Op.START_CMD:
                    pass  # bookkeeping, no runtime effect

                case Op.NOP:
                    pass

                case Op.CONCAT_STK:
                    # Pop N items, concatenate with spaces (like `concat`)
                    argc = instr.operands[0] if instr.operands else 2
                    if isinstance(argc, str):
                        argc = int(argc)
                    items = [stack.pop() for _ in range(min(argc, len(stack)))]
                    items.reverse()
                    # concat trims leading/trailing whitespace from each item
                    # and joins with single spaces
                    result_parts = []
                    for item in items:
                        s = item.strip()
                        if s:
                            result_parts.append(s)
                    stack.append(" ".join(result_parts))

                case Op.TRY_CVT_TO_NUMERIC:
                    # Convert TOS to its canonical numeric form if
                    # it looks like a number.  This mirrors tclsh's
                    # tryCvtToNumeric instruction which normalises
                    # 0o/0x/0b prefixed integers and floats.
                    # Note: boolean strings (true/false/yes/no) are
                    # NOT converted — that only happens in boolean
                    # contexts (&&, ||, !, if, etc.).
                    if stack:
                        _val = stack[-1]
                        _stripped = _val.strip()
                        if _stripped:
                            try:
                                stack[-1] = str(int(_stripped, 0))
                            except (ValueError, TypeError):
                                # Tcl 9.0 doesn't use 0-prefix for octal
                                # (only 0o), so try decimal for leading zeros
                                try:
                                    stack[-1] = str(int(_stripped, 10))
                                except (ValueError, TypeError):
                                    try:
                                        fv = float(_stripped)
                                        if math.isinf(fv):
                                            stack[-1] = "-Inf" if fv < 0 else "Inf"
                                        elif math.isnan(fv):
                                            stack[-1] = "NaN"
                                        else:
                                            stack[-1] = repr(fv)
                                    except (ValueError, TypeError):
                                        pass  # not numeric — leave as-is

                case Op.NOT:
                    # Logical not — Tcl uses "cannot use non-numeric string"
                    # error instead of "expected boolean value"
                    val = stack.pop() if stack else ""
                    try:
                        stack.append("0" if _parse_boolean(val) else "1")
                    except TclError:
                        raise TclError(
                            f'cannot use non-numeric string "{val}" as operand of "!"'
                        ) from None

                case Op.OVER:
                    # Duplicate the element below TOS (copy stack[-2])
                    if len(stack) >= 2:
                        stack.append(stack[-2])
                    else:
                        stack.append("")

                case _:
                    pass  # ignore unhandled ops

            pc += 1

        # Fell off the end without a DONE
        return TclResult(code=ReturnCode.OK, value=stack[-1] if stack else last_result)


# Brace matching


def _braces_balanced(value: str) -> bool:
    """Check whether the first ``{`` and last ``}`` are a matching pair.

    Returns True only when the opening brace at position 0 is balanced
    with the closing brace at the end of *value* — i.e. every inner
    brace is paired before reaching the final ``}``.
    """
    depth = 0
    for i, ch in enumerate(value):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        if depth == 0:
            return i == len(value) - 1
    return False


# List helpers


def _parse_lreplace_index(idx_str: str, length: int) -> int:
    """Parse a Tcl index for lreplace/linsert opcodes."""
    idx_str = idx_str.strip()
    if idx_str == "end":
        return length - 1
    if idx_str.startswith("end-"):
        return length - 1 - int(idx_str[4:])
    if idx_str.startswith("end+"):
        return length - 1 + int(idx_str[4:])
    try:
        return int(idx_str)
    except ValueError:
        pass
    for op in ("+", "-"):
        if op in idx_str[1:]:
            pos = idx_str.index(op, 1)
            left = int(idx_str[:pos])
            right = int(idx_str[pos + 1 :])
            return left + right if op == "+" else left - right
    raise TclError(f'bad index "{idx_str}": must be integer?[+-]integer? or end?[+-]integer?')


def _split_list(text: str) -> list[str]:
    """Split a Tcl list string into elements.

    Raises :class:`TclError` for malformed list syntax (unmatched braces,
    extra characters after close-brace/close-quote, etc.).
    """
    result: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        while i < n and text[i] in " \t\n\r":
            i += 1
        if i >= n:
            break
        if text[i] == "{":
            level = 1
            i += 1
            start = i
            while i < n and level > 0:
                if text[i] == "\\":
                    i += 2
                    continue
                if text[i] == "{":
                    level += 1
                elif text[i] == "}":
                    level -= 1
                i += 1
            if level > 0:
                raise TclError("unmatched open brace in list")
            result.append(text[start : i - 1])
            # After close brace, next char must be whitespace or end
            if i < n and text[i] not in " \t\n\r":
                ch = text[i]
                raise TclError(f'list element in braces followed by "{ch}" instead of space')
        elif text[i] == '"':
            i += 1
            start = i
            while i < n and text[i] != '"':
                if text[i] == "\\":
                    i += 1
                i += 1
            if i >= n:
                raise TclError("unmatched open quote in list")
            result.append(text[start:i])
            i += 1
            # After close quote, next char must be whitespace or end
            if i < n and text[i] not in " \t\n\r":
                ch = text[i]
                raise TclError(f'list element in quotes followed by "{ch}" instead of space')
        else:
            start = i
            while i < n and text[i] not in " \t\n\r":
                if text[i] == "\\":
                    i += 1
                i += 1
            result.append(text[start:i])
    return result


def _list_escape(s: str) -> str:
    """Escape a string for inclusion in a Tcl list.

    Uses brace quoting when the braces would be balanced (accounting
    for backslash-escaped braces), and falls back to backslash escaping
    otherwise — matching Tcl's own list representation rules.
    """
    if not s:
        return "{}"

    # Characters that need quoting in a Tcl list element
    _NEEDS_QUOTING = frozenset(' \t\n\r{}[]$;"\\')

    needs_quote = False
    for ch in s:
        if ch in _NEEDS_QUOTING:
            needs_quote = True
            break
    # Leading # also needs quoting (would start a comment)
    if s[0] == "#":
        needs_quote = True

    if not needs_quote:
        return s

    # Try brace quoting — only works when braces are balanced
    # (after accounting for all backslash-escaped chars) and the string
    # contains no backslash-newline sequences or trailing backslash.
    can_brace = True
    depth = 0
    i = 0
    n = len(s)
    while i < n:
        ch = s[i]
        if ch == "\\":
            if i + 1 < n:
                if s[i + 1] == "\n":
                    # Backslash-newline → unsafe for brace quoting
                    can_brace = False
                    break
                # Skip escaped char (\{, \}, \\, etc. don't affect depth)
                i += 2
                continue
            else:
                # Trailing backslash → unsafe for brace quoting
                can_brace = False
                break
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth < 0:
                can_brace = False
                break
        i += 1
    if depth != 0:
        can_brace = False

    if can_brace:
        return "{" + s + "}"

    # Fall back to backslash escaping
    out: list[str] = []
    for ch in s:
        if ch in '{}[]$";\\':
            out.append("\\" + ch)
        elif ch == " ":
            out.append("\\ ")
        elif ch == "\t":
            out.append("\\t")
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "#" and not out:
            out.append("\\#")
        else:
            out.append(ch)
    return "".join(out)


# Index parsing


def _parse_index(idx_str: str, length: int) -> int:
    """Parse a Tcl index (int, ``end``, ``end-N``, ``int+int``, ``int-int``)."""
    idx_str = idx_str.strip()
    if idx_str == "end":
        return length - 1
    if idx_str.startswith("end-"):
        try:
            return length - 1 - int(idx_str[4:])
        except ValueError:
            pass
    elif idx_str.startswith("end+"):
        try:
            return length - 1 + int(idx_str[4:])
        except ValueError:
            pass
    else:
        try:
            return int(idx_str)
        except ValueError:
            pass
        # Try integer+integer or integer-integer
        for op in ("+", "-"):
            if op in idx_str[1:]:  # skip leading sign
                pos = idx_str.index(op, 1)
                try:
                    left = int(idx_str[:pos])
                    right = int(idx_str[pos + 1 :])
                    return left + right if op == "+" else left - right
                except ValueError:
                    pass
    raise TclError(
        f'bad index "{idx_str}": must be integer?[+-]integer? or end?[+-]integer?'
    ) from None


# Dict helpers


def _dict_from_list(text: str) -> dict[str, str]:
    """Parse a Tcl dict (key/value list) into a Python dict."""
    elements = _split_list(text)
    if len(elements) % 2 != 0:
        raise TclError("missing value to go with key")
    result: dict[str, str] = {}
    for i in range(0, len(elements), 2):
        result[elements[i]] = elements[i + 1]
    return result


def _nested_get(d: dict[str, str], keys: list[str]) -> str:
    """Walk a chain of keys through nested dicts, returning the final value."""
    current: str = " ".join(_list_escape(k) + " " + _list_escape(v) for k, v in d.items())
    for key in keys:
        inner = _dict_from_list(current)
        if key not in inner:
            raise TclError(f'key "{key}" not known in dictionary')
        current = inner[key]
    return current


def _nested_exists(d: dict[str, str], keys: list[str]) -> bool:
    """Check whether a chain of keys exists in nested dicts."""
    current: str = " ".join(_list_escape(k) + " " + _list_escape(v) for k, v in d.items())
    for key in keys:
        try:
            inner = _dict_from_list(current)
        except TclError:
            return False
        if key not in inner:
            return False
        current = inner[key]
    return True
