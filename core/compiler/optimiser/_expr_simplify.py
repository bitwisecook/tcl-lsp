"""Expression AST manipulation and simplification for the optimiser."""

from __future__ import annotations

import re

from ...common.dialect import active_dialect
from ...parsing.expr_lexer import ExprTokenType, tokenise_expr
from ...parsing.expr_parser import parse_expr
from ..expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from ..token_helpers import parse_decimal_int as _parse_decimal_int
from ..types import TclType, TypeKind, TypeLattice
from ._helpers import _parse_string_length_arg

# --- Constants ---

_BIN_PRECEDENCE: dict[BinOp, int] = {
    BinOp.OR: 4,
    BinOp.WORD_OR: 4,
    BinOp.AND: 6,
    BinOp.WORD_AND: 6,
    BinOp.BIT_OR: 8,
    BinOp.BIT_XOR: 10,
    BinOp.BIT_AND: 12,
    BinOp.EQ: 14,
    BinOp.NE: 14,
    BinOp.STR_EQ: 14,
    BinOp.STR_NE: 14,
    BinOp.CONTAINS: 14,
    BinOp.STARTS_WITH: 14,
    BinOp.ENDS_WITH: 14,
    BinOp.STR_EQUALS: 14,
    BinOp.MATCHES_GLOB: 14,
    BinOp.MATCHES_REGEX: 14,
    BinOp.LT: 16,
    BinOp.LE: 16,
    BinOp.GT: 16,
    BinOp.GE: 16,
    BinOp.STR_LT: 16,
    BinOp.STR_LE: 16,
    BinOp.STR_GT: 16,
    BinOp.STR_GE: 16,
    BinOp.IN: 16,
    BinOp.NI: 16,
    BinOp.LSHIFT: 18,
    BinOp.RSHIFT: 18,
    BinOp.ADD: 20,
    BinOp.SUB: 20,
    BinOp.MUL: 22,
    BinOp.DIV: 22,
    BinOp.MOD: 22,
    BinOp.POW: 23,
}
_RIGHT_ASSOC_BINOPS = frozenset({BinOp.POW})

_DEMORGAN_DUAL: dict[BinOp, BinOp] = {
    BinOp.AND: BinOp.OR,
    BinOp.OR: BinOp.AND,
    BinOp.WORD_AND: BinOp.WORD_OR,
    BinOp.WORD_OR: BinOp.WORD_AND,
}

_COMPARISON_INVERSION: dict[BinOp, BinOp] = {
    BinOp.EQ: BinOp.NE,
    BinOp.NE: BinOp.EQ,
    BinOp.LT: BinOp.GE,
    BinOp.GE: BinOp.LT,
    BinOp.GT: BinOp.LE,
    BinOp.LE: BinOp.GT,
    BinOp.STR_EQ: BinOp.STR_NE,
    BinOp.STR_NE: BinOp.STR_EQ,
    BinOp.STR_LT: BinOp.STR_GE,
    BinOp.STR_GE: BinOp.STR_LT,
    BinOp.STR_GT: BinOp.STR_LE,
    BinOp.STR_LE: BinOp.STR_GT,
    BinOp.IN: BinOp.NI,
    BinOp.NI: BinOp.IN,
}

_BOOLEAN_OPS = frozenset(
    {
        BinOp.AND,
        BinOp.OR,
        BinOp.WORD_AND,
        BinOp.WORD_OR,
        BinOp.EQ,
        BinOp.NE,
        BinOp.LT,
        BinOp.LE,
        BinOp.GT,
        BinOp.GE,
        BinOp.STR_EQ,
        BinOp.STR_NE,
        BinOp.STR_LT,
        BinOp.STR_LE,
        BinOp.STR_GT,
        BinOp.STR_GE,
        BinOp.IN,
        BinOp.NI,
        BinOp.CONTAINS,
        BinOp.STARTS_WITH,
        BinOp.ENDS_WITH,
        BinOp.STR_EQUALS,
        BinOp.MATCHES_GLOB,
        BinOp.MATCHES_REGEX,
    }
)

_REGEX_META_RE = re.compile(r"[.*+?\[\](){}|\\]")
_GLOB_META_RE = re.compile(r"[*?\[\]]")
_BOOLEAN_WORDS = frozenset(("true", "false", "yes", "no", "on", "off"))

# --- Helper functions ---


def _try_fold_expr(expr: str) -> str | None:
    from ..tcl_expr_eval import eval_tcl_expr_str, format_tcl_value

    value = eval_tcl_expr_str(expr)
    if value is None:
        return None
    return format_tcl_value(value)


def _expr_has_command_subst(node: ExprNode) -> bool:
    """Return True when an expression tree contains command substitution."""
    match node:
        case ExprCommand():
            return True
        case ExprRaw(text=text):
            return "[" in text
        case ExprBinary(left=left, right=right):
            return _expr_has_command_subst(left) or _expr_has_command_subst(right)
        case ExprUnary(operand=operand):
            return _expr_has_command_subst(operand)
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            return (
                _expr_has_command_subst(cond)
                or _expr_has_command_subst(tb)
                or _expr_has_command_subst(fb)
            )
        case ExprCall(args=args):
            return any(_expr_has_command_subst(a) for a in args)
        case _:
            return False


def _int_literal_value(node: ExprNode) -> int | None:
    if not isinstance(node, ExprLiteral):
        return None
    parsed = _parse_decimal_int(node.text)
    if parsed is None:
        return None
    return int(parsed)


def _make_int_literal(value: int) -> ExprLiteral:
    text = str(value)
    return ExprLiteral(text=text, start=0, end=max(0, len(text) - 1))


def _fold_binary_int_literals(op: BinOp, left: ExprNode, right: ExprNode) -> ExprLiteral | None:
    lv = _int_literal_value(left)
    rv = _int_literal_value(right)
    if lv is None or rv is None:
        return None

    value: int | None = None
    match op:
        case BinOp.ADD:
            value = lv + rv
        case BinOp.SUB:
            value = lv - rv
        case BinOp.MUL:
            value = lv * rv
        case BinOp.BIT_AND:
            value = lv & rv
        case BinOp.BIT_OR:
            value = lv | rv
        case BinOp.BIT_XOR:
            value = lv ^ rv
        case BinOp.LSHIFT if rv >= 0:
            value = lv << rv
        case BinOp.RSHIFT if rv >= 0:
            value = lv >> rv
        case _:
            return None
    return _make_int_literal(value)


def _collect_add_terms(
    node: ExprNode,
    terms: list[ExprNode],
) -> int:
    if isinstance(node, ExprBinary):
        if node.op is BinOp.ADD:
            return _collect_add_terms(node.left, terms) + _collect_add_terms(node.right, terms)
        if node.op is BinOp.SUB:
            rhs_int = _int_literal_value(node.right)
            if rhs_int is not None:
                return _collect_add_terms(node.left, terms) - rhs_int

    int_val = _int_literal_value(node)
    if int_val is not None:
        return int_val

    terms.append(node)
    return 0


def _build_add_expr(terms: list[ExprNode], constant: int) -> ExprNode:
    if not terms:
        return _make_int_literal(constant)

    result = terms[0]
    for term in terms[1:]:
        result = ExprBinary(op=BinOp.ADD, left=result, right=term)

    if constant > 0:
        return ExprBinary(op=BinOp.ADD, left=result, right=_make_int_literal(constant))
    if constant < 0:
        return ExprBinary(op=BinOp.SUB, left=result, right=_make_int_literal(-constant))
    return result


def _collect_mul_terms(
    node: ExprNode,
    terms: list[ExprNode],
) -> int:
    if isinstance(node, ExprBinary) and node.op is BinOp.MUL:
        return _collect_mul_terms(node.left, terms) * _collect_mul_terms(node.right, terms)

    int_val = _int_literal_value(node)
    if int_val is not None:
        return int_val

    terms.append(node)
    return 1


def _build_mul_expr(terms: list[ExprNode], constant: int) -> ExprNode:
    if constant == 0:
        return _make_int_literal(0)
    if not terms:
        return _make_int_literal(constant)

    result = terms[0]
    for term in terms[1:]:
        result = ExprBinary(op=BinOp.MUL, left=result, right=term)

    if constant != 1:
        return ExprBinary(op=BinOp.MUL, left=result, right=_make_int_literal(constant))
    return result


def _is_boolean_expr(node: ExprNode) -> bool:
    """Return True if *node* is known to produce a boolean (0/1) result."""
    match node:
        case ExprBinary(op=op) if op in _BOOLEAN_OPS:
            return True
        case ExprUnary(op=UnaryOp.NOT | UnaryOp.WORD_NOT):
            return True
        case ExprLiteral(text=text):
            return text in ("0", "1", "true", "false")
        case _:
            return False


def _nodes_equal(a: ExprNode, b: ExprNode) -> bool:
    """Structural equality ignoring source positions."""
    match a, b:
        case ExprLiteral(text=t1), ExprLiteral(text=t2):
            return t1 == t2
        case ExprString(text=t1), ExprString(text=t2):
            return t1 == t2
        case ExprVar(name=n1), ExprVar(name=n2):
            return n1 == n2
        case ExprCommand(text=t1), ExprCommand(text=t2):
            return t1 == t2
        case ExprBinary(op=o1, left=l1, right=r1), ExprBinary(op=o2, left=l2, right=r2):
            return o1 is o2 and _nodes_equal(l1, l2) and _nodes_equal(r1, r2)
        case ExprUnary(op=o1, operand=a1), ExprUnary(op=o2, operand=a2):
            return o1 is o2 and _nodes_equal(a1, a2)
        case (
            ExprTernary(condition=c1, true_branch=t1, false_branch=f1),
            ExprTernary(condition=c2, true_branch=t2, false_branch=f2),
        ):
            return _nodes_equal(c1, c2) and _nodes_equal(t1, t2) and _nodes_equal(f1, f2)
        case ExprCall(function=fn1, args=a1), ExprCall(function=fn2, args=a2):
            return (
                fn1 == fn2
                and len(a1) == len(a2)
                and all(_nodes_equal(x, y) for x, y in zip(a1, a2))
            )
        case ExprRaw(text=t1), ExprRaw(text=t2):
            return t1 == t2
        case _:
            return False


def _strip_expr_string(text: str) -> str:
    """Strip surrounding braces or quotes from an ExprString.text value."""
    if len(text) >= 2 and (
        (text[0] == "{" and text[-1] == "}") or (text[0] == '"' and text[-1] == '"')
    ):
        return text[1:-1]
    return text


def _is_numeric_string_value(value: str) -> bool:
    """Return True if a string literal is numeric/boolean-like for expr."""
    stripped = value.strip()
    if not stripped:
        return False
    if _parse_decimal_int(stripped) is not None:
        return True
    if stripped.lower() in _BOOLEAN_WORDS:
        return True
    try:
        float(stripped)
    except ValueError:
        return False
    return True


def _known_var_type_for_expr_node(
    node: ExprNode,
    *,
    ssa_uses: dict[str, int] | None,
    types: dict[tuple[str, int], TypeLattice] | None,
) -> TclType | None:
    """Return known TclType for an ExprVar, if available."""
    if not isinstance(node, ExprVar) or ssa_uses is None or types is None:
        return None
    ver = ssa_uses.get(node.name, 0)
    if ver <= 0:
        return None
    inferred = types.get((node.name, ver))
    if inferred is None or inferred.kind is not TypeKind.KNOWN:
        return None
    return inferred.tcl_type


def _is_definitely_string_expr_node(
    node: ExprNode,
    *,
    ssa_uses: dict[str, int] | None,
    types: dict[tuple[str, int], TypeLattice] | None,
) -> bool:
    """Return True when expr type is certainly string at this program point."""
    if isinstance(node, ExprString):
        return True
    known = _known_var_type_for_expr_node(node, ssa_uses=ssa_uses, types=types)
    return known is TclType.STRING


def _rewrite_eq_ne_string_compare_node(
    node: ExprNode,
    *,
    ssa_uses: dict[str, int] | None,
    types: dict[tuple[str, int], TypeLattice] | None,
) -> tuple[ExprNode, bool]:
    """Rewrite ``==``/``!=`` string compares to ``eq``/``ne``."""
    match node:
        case ExprBinary(op=op, left=left, right=right):
            left_new, left_changed = _rewrite_eq_ne_string_compare_node(
                left,
                ssa_uses=ssa_uses,
                types=types,
            )
            right_new, right_changed = _rewrite_eq_ne_string_compare_node(
                right,
                ssa_uses=ssa_uses,
                types=types,
            )
            changed = left_changed or right_changed

            if op not in (BinOp.EQ, BinOp.NE):
                if changed:
                    return ExprBinary(op=op, left=left_new, right=right_new), True
                return node, False

            left_is_str = isinstance(left_new, ExprString)
            right_is_str = isinstance(right_new, ExprString)

            if not left_is_str and not right_is_str:
                # Neither side is a literal — check SSA types.
                # Require *both* operands to be definitively string-typed
                # to avoid changing semantics for mixed string/numeric
                # comparisons (e.g. STRING vs INT).
                left_def_str = _is_definitely_string_expr_node(
                    left_new,
                    ssa_uses=ssa_uses,
                    types=types,
                )
                right_def_str = _is_definitely_string_expr_node(
                    right_new,
                    ssa_uses=ssa_uses,
                    types=types,
                )
                if not (left_def_str and right_def_str):
                    if changed:
                        return ExprBinary(op=op, left=left_new, right=right_new), True
                    return node, False
            elif left_is_str:
                assert isinstance(left_new, ExprString)
                literal_text = _strip_expr_string(left_new.text)
                if _is_numeric_string_value(literal_text) and not _is_definitely_string_expr_node(
                    right_new,
                    ssa_uses=ssa_uses,
                    types=types,
                ):
                    if changed:
                        return ExprBinary(op=op, left=left_new, right=right_new), True
                    return node, False
            else:
                assert isinstance(right_new, ExprString)
                literal_text = _strip_expr_string(right_new.text)
                if _is_numeric_string_value(literal_text) and not _is_definitely_string_expr_node(
                    left_new,
                    ssa_uses=ssa_uses,
                    types=types,
                ):
                    if changed:
                        return ExprBinary(op=op, left=left_new, right=right_new), True
                    return node, False

            new_op = BinOp.STR_EQ if op is BinOp.EQ else BinOp.STR_NE
            return ExprBinary(op=new_op, left=left_new, right=right_new), True

        case ExprUnary(op=op, operand=operand):
            operand_new, changed = _rewrite_eq_ne_string_compare_node(
                operand,
                ssa_uses=ssa_uses,
                types=types,
            )
            if changed:
                return ExprUnary(op=op, operand=operand_new), True
            return node, False

        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            cond_new, c_changed = _rewrite_eq_ne_string_compare_node(
                cond,
                ssa_uses=ssa_uses,
                types=types,
            )
            tb_new, t_changed = _rewrite_eq_ne_string_compare_node(
                tb,
                ssa_uses=ssa_uses,
                types=types,
            )
            fb_new, f_changed = _rewrite_eq_ne_string_compare_node(
                fb,
                ssa_uses=ssa_uses,
                types=types,
            )
            if c_changed or t_changed or f_changed:
                return ExprTernary(
                    condition=cond_new, true_branch=tb_new, false_branch=fb_new
                ), True
            return node, False

        case ExprCall(function=function, args=args, start=start, end=end):
            changed = False
            new_args: list[ExprNode] = []
            for arg in args:
                arg_new, arg_changed = _rewrite_eq_ne_string_compare_node(
                    arg,
                    ssa_uses=ssa_uses,
                    types=types,
                )
                changed = changed or arg_changed
                new_args.append(arg_new)
            if changed:
                return ExprCall(function=function, args=tuple(new_args), start=start, end=end), True
            return node, False

        case _:
            return node, False


def _try_eq_ne_string_compare_simplify_expr(
    expr: str,
    *,
    ssa_uses: dict[str, int] | None = None,
    types: dict[tuple[str, int], TypeLattice] | None = None,
) -> tuple[str, bool]:
    """Simplify string compares from numeric to string operators."""
    stripped = expr.strip()
    parsed = parse_expr(stripped, dialect=active_dialect())
    if isinstance(parsed, ExprRaw):
        return expr, False
    rewritten_node, changed = _rewrite_eq_ne_string_compare_node(
        parsed,
        ssa_uses=ssa_uses,
        types=types,
    )
    if not changed:
        return expr, False
    rendered = _render_expr_for_rewrite(rewritten_node)
    if rendered == stripped:
        return expr, False
    return rendered, True


def _simplify_regex_match(
    left: ExprNode,
    pattern: str,
) -> ExprBinary | None:
    """Try to reduce ``matches_regex`` to a simpler string operator."""
    if not pattern:
        return None

    has_start = pattern.startswith("^")
    has_end = pattern.endswith("$") and not pattern.endswith("\\$")

    literal = pattern
    if has_start:
        literal = literal[1:]
    if has_end:
        literal = literal[:-1]

    if not literal or _REGEX_META_RE.search(literal):
        return None

    if has_start and has_end:
        new_op = BinOp.STR_EQUALS
    elif has_start:
        new_op = BinOp.STARTS_WITH
    elif has_end:
        new_op = BinOp.ENDS_WITH
    else:
        new_op = BinOp.CONTAINS

    return ExprBinary(
        op=new_op,
        left=left,
        right=ExprString(text="{" + literal + "}", start=0, end=0),
    )


def _simplify_glob_match(
    left: ExprNode,
    pattern: str,
) -> ExprBinary | None:
    """Try to reduce ``matches_glob`` to a simpler string operator."""
    if not pattern:
        return None

    has_lead = pattern.startswith("*")
    has_trail = pattern.endswith("*")

    literal = pattern
    if has_lead:
        literal = literal[1:]
    if has_trail:
        literal = literal[:-1]

    if not literal or _GLOB_META_RE.search(literal):
        return None

    if has_lead and has_trail:
        new_op = BinOp.CONTAINS
    elif has_lead:
        new_op = BinOp.ENDS_WITH
    elif has_trail:
        new_op = BinOp.STARTS_WITH
    else:
        new_op = BinOp.STR_EQUALS

    return ExprBinary(
        op=new_op,
        left=left,
        right=ExprString(text="{" + literal + "}", start=0, end=0),
    )


def _simplify_expr_node(node: ExprNode, *, bool_context: bool = False) -> ExprNode:
    _simp = _simplify_expr_node  # local alias for brevity

    match node:
        case ExprUnary(op=op, operand=operand):
            simp_operand = _simp(operand)
            if op is UnaryOp.POS:
                return simp_operand
            if op is UnaryOp.NEG:
                if isinstance(simp_operand, ExprUnary) and simp_operand.op is UnaryOp.NEG:
                    return simp_operand.operand
                lit_val = _int_literal_value(simp_operand)
                if lit_val is not None:
                    return _make_int_literal(-lit_val)
            if op in (UnaryOp.NOT, UnaryOp.WORD_NOT):
                # !!x -> x (when x already boolean or in boolean context)
                if isinstance(simp_operand, ExprUnary) and simp_operand.op in (
                    UnaryOp.NOT,
                    UnaryOp.WORD_NOT,
                ):
                    if bool_context or _is_boolean_expr(simp_operand.operand):
                        return simp_operand.operand
                # !($a == $b) -> $a != $b  (comparison inversion)
                if isinstance(simp_operand, ExprBinary):
                    inv = _COMPARISON_INVERSION.get(simp_operand.op)
                    if inv is not None:
                        return ExprBinary(op=inv, left=simp_operand.left, right=simp_operand.right)
                    # De Morgan's: !(a && b) -> !a || !b, !(a || b) -> !a && !b
                    dual = _DEMORGAN_DUAL.get(simp_operand.op)
                    if dual is not None:
                        return ExprBinary(
                            op=dual,
                            left=ExprUnary(op=op, operand=simp_operand.left),
                            right=ExprUnary(op=op, operand=simp_operand.right),
                        )
                # ~(~x) -> x
            if op is UnaryOp.BIT_NOT:
                if isinstance(simp_operand, ExprUnary) and simp_operand.op is UnaryOp.BIT_NOT:
                    return simp_operand.operand
            return ExprUnary(op=op, operand=simp_operand)

        case ExprBinary(op=op, left=left, right=right):
            simp_left = _simp(left)
            simp_right = _simp(right)

            if folded := _fold_binary_int_literals(op, simp_left, simp_right):
                return folded

            lv = _int_literal_value(simp_left)
            rv = _int_literal_value(simp_right)

            # Additive identities / reassociation
            if op in (BinOp.ADD, BinOp.SUB):
                terms: list[ExprNode] = []
                constant = _collect_add_terms(
                    ExprBinary(op=op, left=simp_left, right=simp_right),
                    terms,
                )
                return _build_add_expr(terms, constant)

            # Multiplicative identities / reassociation
            if op is BinOp.MUL:
                terms = []
                constant = _collect_mul_terms(
                    ExprBinary(op=op, left=simp_left, right=simp_right),
                    terms,
                )
                return _build_mul_expr(terms, constant)

            # Division: x / 1 -> x
            if op is BinOp.DIV:
                if rv == 1:
                    return simp_left

            # Exponentiation
            if op is BinOp.POW:
                if rv == 0:
                    return _make_int_literal(1)
                if rv == 1:
                    return simp_left
                # O113: strength reduction -- x ** 2 -> x * x
                if rv == 2:
                    return ExprBinary(op=BinOp.MUL, left=simp_left, right=simp_left)

            # Shift: x << 0 -> x, x >> 0 -> x
            if op in (BinOp.LSHIFT, BinOp.RSHIFT):
                if rv == 0:
                    return simp_left

            # Bitwise AND: x & 0 -> 0, 0 & x -> 0
            if op is BinOp.BIT_AND:
                if rv == 0 or lv == 0:
                    return _make_int_literal(0)

            # Bitwise OR / XOR: x | 0 -> x, x ^ 0 -> x
            if op in (BinOp.BIT_OR, BinOp.BIT_XOR):
                if rv == 0:
                    return simp_left
                if lv == 0:
                    return simp_right

            # Modulo: x % 1 -> 0
            if op is BinOp.MOD:
                if rv == 1:
                    return _make_int_literal(0)
                # O113: strength reduction -- x % (2^N) -> x & (2^N - 1)
                if rv is not None and rv > 1 and (rv & (rv - 1)) == 0:
                    return ExprBinary(
                        op=BinOp.BIT_AND,
                        left=simp_left,
                        right=_make_int_literal(rv - 1),
                    )

            # Logical AND
            if op in (BinOp.AND, BinOp.WORD_AND):
                if rv == 0 or lv == 0:
                    return _make_int_literal(0)
                if rv == 1:
                    return simp_left
                if lv == 1:
                    return simp_right

            # Logical OR
            if op in (BinOp.OR, BinOp.WORD_OR):
                if rv == 1 or lv == 1:
                    return _make_int_literal(1)
                if rv == 0:
                    return simp_left
                if lv == 0:
                    return simp_right

            # Self-comparison tautologies (safe for integers)
            if _nodes_equal(simp_left, simp_right):
                if op in (BinOp.EQ, BinOp.LE, BinOp.GE, BinOp.STR_EQ, BinOp.STR_LE, BinOp.STR_GE):
                    return _make_int_literal(1)
                if op in (BinOp.NE, BinOp.LT, BinOp.GT, BinOp.STR_NE, BinOp.STR_LT, BinOp.STR_GT):
                    return _make_int_literal(0)
                if op is BinOp.SUB:
                    return _make_int_literal(0)
                if op is BinOp.BIT_XOR:
                    return _make_int_literal(0)

            # Pattern match simplification (matches_regex / matches_glob)
            if op is BinOp.MATCHES_REGEX and isinstance(simp_right, ExprString):
                result = _simplify_regex_match(simp_left, _strip_expr_string(simp_right.text))
                if result is not None:
                    return result

            if op is BinOp.MATCHES_GLOB and isinstance(simp_right, ExprString):
                result = _simplify_glob_match(simp_left, _strip_expr_string(simp_right.text))
                if result is not None:
                    return result

            return ExprBinary(op=op, left=simp_left, right=simp_right)

        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            simp_cond = _simp(cond, bool_context=True)
            simp_tb = _simp(tb, bool_context=bool_context)
            simp_fb = _simp(fb, bool_context=bool_context)

            # Constant condition: fold to the taken branch.
            cond_val = _int_literal_value(simp_cond)
            if cond_val is not None:
                return simp_tb if cond_val else simp_fb

            # x ? a : a -> a  (identical branches)
            if _nodes_equal(simp_tb, simp_fb):
                return simp_tb

            # x ? 1 : 0 -> x  (in boolean context, or when x is already boolean)
            if _int_literal_value(simp_tb) == 1 and _int_literal_value(simp_fb) == 0:
                if bool_context or _is_boolean_expr(simp_cond):
                    return simp_cond

            # x ? 0 : 1 -> !x
            if _int_literal_value(simp_tb) == 0 and _int_literal_value(simp_fb) == 1:
                return ExprUnary(op=UnaryOp.NOT, operand=simp_cond)

            # !c ? a : b -> c ? b : a  (remove negation from condition)
            if isinstance(simp_cond, ExprUnary) and simp_cond.op in (UnaryOp.NOT, UnaryOp.WORD_NOT):
                return ExprTernary(
                    condition=simp_cond.operand,
                    true_branch=simp_fb,
                    false_branch=simp_tb,
                )

            return ExprTernary(
                condition=simp_cond,
                true_branch=simp_tb,
                false_branch=simp_fb,
            )

        case ExprCall(function=function, args=args, start=start, end=end):
            return ExprCall(
                function=function,
                args=tuple(_simp(arg) for arg in args),
                start=start,
                end=end,
            )

        case _:
            return node


def _render_expr_for_rewrite(node: ExprNode, parent_prec: int = 0) -> str:
    match node:
        case ExprLiteral(text=text):
            return text
        case ExprString(text=text):
            return text
        case ExprVar(text=text):
            return text
        case ExprCommand(text=text):
            return text
        case ExprRaw(text=text):
            return text
        case ExprUnary(op=op, operand=operand):
            prec = 24
            operand_text = _render_expr_for_rewrite(operand, prec)
            if op in (UnaryOp.WORD_NOT,):
                text = f"{op.value} {operand_text}"
            else:
                text = f"{op.value}{operand_text}"
            if prec < parent_prec:
                return f"({text})"
            return text
        case ExprBinary(op=op, left=left, right=right):
            prec = _BIN_PRECEDENCE.get(op, 20)
            if op in _RIGHT_ASSOC_BINOPS:
                left_prec = prec + 1
                right_prec = prec
            else:
                left_prec = prec
                right_prec = prec + 1
            left_text = _render_expr_for_rewrite(left, left_prec)
            right_text = _render_expr_for_rewrite(right, right_prec)
            text = f"{left_text} {op.value} {right_text}"
            if prec < parent_prec:
                return f"({text})"
            return text
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            prec = 2
            text = (
                f"{_render_expr_for_rewrite(cond, prec + 1)} ? "
                f"{_render_expr_for_rewrite(tb, 0)} : "
                f"{_render_expr_for_rewrite(fb, prec)}"
            )
            if prec < parent_prec:
                return f"({text})"
            return text
        case ExprCall(function=function, args=args):
            rendered_args = ", ".join(_render_expr_for_rewrite(a, 0) for a in args)
            return f"{function}({rendered_args})"
        case ExprRaw(text=text):
            return text
        case _:
            return str(node)


def _simplify_to_fixpoint(
    node: ExprNode,
    *,
    bool_context: bool = False,
    max_iters: int = 4,
) -> ExprNode:
    """Apply _simplify_expr_node repeatedly until no further change."""
    for _ in range(max_iters):
        simplified = _simplify_expr_node(node, bool_context=bool_context)
        if simplified == node:
            break
        node = simplified
    return node


def _try_demorgan_node(node: ExprNode) -> ExprNode | None:
    """Apply one step of De Morgan's law (either direction)."""
    # Forward: !(a op b) -> !a dual_op !b
    if isinstance(node, ExprUnary) and node.op in (UnaryOp.NOT, UnaryOp.WORD_NOT):
        if isinstance(node.operand, ExprBinary):
            dual = _DEMORGAN_DUAL.get(node.operand.op)
            if dual is not None:
                return ExprBinary(
                    op=dual,
                    left=ExprUnary(op=node.op, operand=node.operand.left),
                    right=ExprUnary(op=node.op, operand=node.operand.right),
                )
    # Reverse: !a op !b -> !(a dual_op b)
    if isinstance(node, ExprBinary) and node.op in _DEMORGAN_DUAL:
        if (
            isinstance(node.left, ExprUnary)
            and node.left.op in (UnaryOp.NOT, UnaryOp.WORD_NOT)
            and isinstance(node.right, ExprUnary)
            and node.right.op in (UnaryOp.NOT, UnaryOp.WORD_NOT)
        ):
            dual = _DEMORGAN_DUAL[node.op]
            return ExprUnary(
                op=node.left.op,
                operand=ExprBinary(op=dual, left=node.left.operand, right=node.right.operand),
            )
    return None


def demorgan_transform(expr: str) -> str | None:
    """Apply one step of De Morgan's law to an expression string.

    Returns the transformed text, or None if the law does not apply.
    """
    expr = expr.strip()
    if not expr:
        return None
    parsed = parse_expr(expr, dialect=active_dialect())
    if isinstance(parsed, ExprRaw):
        return None
    result = _try_demorgan_node(parsed)
    if result is None:
        return None
    rendered = _render_expr_for_rewrite(result)
    original = _render_expr_for_rewrite(parsed)
    if rendered == original:
        return None
    return rendered


def invert_expression(expr: str) -> str | None:
    """Negate an expression and simplify via De Morgan's + comparison inversion.

    Returns the inverted text, or None if the expression cannot be parsed
    or the result is identical to the input.
    """
    expr = expr.strip()
    if not expr:
        return None
    parsed = parse_expr(expr, dialect=active_dialect())
    if isinstance(parsed, ExprRaw):
        return None
    negated = ExprUnary(op=UnaryOp.NOT, operand=parsed)
    simplified = _simplify_to_fixpoint(negated, bool_context=True)
    rendered = _render_expr_for_rewrite(simplified)
    original = _render_expr_for_rewrite(parsed)
    if rendered == original:
        return None
    return rendered


def _try_simplify_pattern_match(parsed: ExprNode) -> ExprNode | None:
    """Apply only the matches_regex/matches_glob -> simpler-op rewrite."""
    if not isinstance(parsed, ExprBinary):
        return None
    if parsed.op is BinOp.MATCHES_REGEX and isinstance(parsed.right, ExprString):
        return _simplify_regex_match(parsed.left, _strip_expr_string(parsed.right.text))
    if parsed.op is BinOp.MATCHES_GLOB and isinstance(parsed.right, ExprString):
        return _simplify_glob_match(parsed.left, _strip_expr_string(parsed.right.text))
    return None


def _instcombine_expr(expr: str, *, bool_context: bool = False) -> tuple[str, bool]:
    stripped = expr.strip()
    parsed = parse_expr(stripped, dialect=active_dialect())
    if isinstance(parsed, ExprRaw):
        return expr, False
    if _expr_has_command_subst(parsed):
        pm = _try_simplify_pattern_match(parsed)
        if pm is not None:
            rewritten = _render_expr_for_rewrite(pm)
            return rewritten, rewritten != stripped
        # O117: [string length $s] == 0 -> $s eq ""
        sl = _simplify_strlen_node(parsed)
        if sl is not None:
            rewritten = _render_expr_for_rewrite(sl)
            return rewritten, rewritten != stripped
        return expr, False

    simplified = _simplify_to_fixpoint(parsed, bool_context=bool_context)
    rewritten = _render_expr_for_rewrite(simplified)
    return rewritten, rewritten != stripped


def _substitute_expr_constants(expr: str, constants: dict[str, str]) -> tuple[str, bool, set[str]]:
    from ...common.naming import normalise_var_name as _normalise_var_name

    pieces: list[str] = []
    cursor = 0
    changed = False
    substituted_names: set[str] = set()

    for tok in tokenise_expr(expr, dialect=active_dialect()):
        if tok.start > cursor:
            pieces.append(expr[cursor : tok.start])

        if tok.type is ExprTokenType.VARIABLE:
            name = _normalise_var_name(tok.text)
            value = constants.get(name)
            if value is not None:
                # Numeric values can be substituted directly; string values
                # must be quoted to be valid in expr context.
                try:
                    int(value)
                    pieces.append(value)
                except (ValueError, OverflowError):
                    try:
                        float(value)
                        pieces.append(value)
                    except (ValueError, OverflowError):
                        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
                        pieces.append(f'"{escaped}"')
                changed = True
                substituted_names.add(name)
            else:
                pieces.append(tok.text)
        else:
            pieces.append(tok.text)

        cursor = tok.end + 1

    if cursor < len(expr):
        pieces.append(expr[cursor:])

    return "".join(pieces), changed, substituted_names


# O113: Strength reduction helpers


def _strength_reduce_node(node: ExprNode) -> ExprNode | None:
    """Apply one level of strength reduction to a top-level expression."""
    if not isinstance(node, ExprBinary):
        return None
    rv = _int_literal_value(node.right)
    if node.op is BinOp.POW and rv == 2:
        return ExprBinary(op=BinOp.MUL, left=node.left, right=node.left)
    if node.op is BinOp.MOD and rv is not None and rv > 1 and (rv & (rv - 1)) == 0:
        return ExprBinary(
            op=BinOp.BIT_AND,
            left=node.left,
            right=_make_int_literal(rv - 1),
        )
    return None


def _try_strength_reduce_expr(expr: str) -> tuple[str, bool]:
    """Apply strength reduction: x**2 -> x*x, x%(2^N) -> x&(2^N-1)."""
    stripped = expr.strip()
    parsed = parse_expr(stripped, dialect=active_dialect())
    if isinstance(parsed, ExprRaw):
        return expr, False
    if _expr_has_command_subst(parsed):
        return expr, False
    reduced = _strength_reduce_node(parsed)
    if reduced is None:
        return expr, False
    rendered = _render_expr_for_rewrite(reduced)
    if rendered == stripped:
        return expr, False
    return rendered, True


# O115: Redundant nested expr helpers


def _try_unwrap_expr_in_expr(expr_text: str) -> str | None:
    """If *expr_text* is entirely ``[expr {E}]``, return ``E``."""
    from ._helpers import _expr_arg_from_expr_command

    stripped = expr_text.strip()
    if not stripped.startswith("[") or not stripped.endswith("]"):
        return None
    cmd_text = stripped[1:-1]
    return _expr_arg_from_expr_command(cmd_text)


# O117: String length zero-check helpers


def _simplify_strlen_node(node: ExprNode) -> ExprNode | None:
    """Simplify ``[string length $s] == 0`` -> ``$s eq ""``, etc."""
    if not isinstance(node, ExprBinary):
        return None

    cmd_node: ExprCommand | None = None
    lit_node: ExprLiteral | None = None
    swapped = False

    if isinstance(node.left, ExprCommand) and isinstance(node.right, ExprLiteral):
        cmd_node = node.left
        lit_node = node.right
    elif isinstance(node.right, ExprCommand) and isinstance(node.left, ExprLiteral):
        cmd_node = node.right
        lit_node = node.left
        swapped = True
    else:
        return None

    if lit_node.text.strip() != "0":
        return None

    arg_text = _parse_string_length_arg(cmd_node.text)
    if arg_text is None:
        return None

    op = node.op
    if swapped:
        _flip = {BinOp.LT: BinOp.GT, BinOp.GT: BinOp.LT, BinOp.LE: BinOp.GE, BinOp.GE: BinOp.LE}
        op = _flip.get(op, op)

    new_op: BinOp | None = None
    if op in (BinOp.EQ, BinOp.LE):
        new_op = BinOp.STR_EQ
    elif op in (BinOp.NE, BinOp.GT):
        new_op = BinOp.STR_NE
    else:
        return None

    return ExprBinary(
        op=new_op,
        left=ExprRaw(text=arg_text),
        right=ExprString(text='""', start=0, end=0),
    )


def _try_strlen_simplify_expr(expr: str) -> tuple[str, bool]:
    """Simplify string length zero-check patterns in an expression."""
    stripped = expr.strip()
    parsed = parse_expr(stripped, dialect=active_dialect())
    if isinstance(parsed, ExprRaw):
        return expr, False
    result = _simplify_strlen_node(parsed)
    if result is None:
        return expr, False
    rendered = _render_expr_for_rewrite(result)
    if rendered == stripped:
        return expr, False
    return rendered, True
