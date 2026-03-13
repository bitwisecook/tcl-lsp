"""Style and best-practice checks (W1xx, W2xx)."""

from __future__ import annotations

import re
from collections.abc import Callable

from ...commands.registry.runtime import (
    ArgRole,
    arg_indices_for_role,
)
from ...common.dialect import active_dialect
from ...common.ranges import position_from_relative, range_from_token, range_from_tokens
from ...compiler.expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
)
from ...parsing.expr_parser import parse_expr
from ...parsing.tokens import SourcePosition, Token, TokenType
from ..semantic_model import CodeFix, Diagnostic, Range, Severity
from ._helpers import (
    _build_file_join_fix,
    _first_arg_name,
    _first_positional_without_terminator,
    _first_token_is_braced,
    _has_substitution,
    _info_exists_arg,
    _is_safe_literal,
    _is_safe_literal_expr,
    _last_literal_set_value_for_var,
    _parse_subst_flags,
    _pos_in_cmd_text,
    _reconstruct_word_from_tokens,
    _resolve_option_terminator_profile,
    _rewrite_string_compare_ops,
    _unset_name_args,
    _upvar_local_name_args,
)

# W100: Unbraced expr


def check_unbraced_expr(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W100: Warn when expr/if/while/for expressions are not braced.

    Unbraced expressions suffer double-substitution: the Tcl parser substitutes
    variables and commands first, then expr evaluates the result again.  This is
    both a performance problem (defeats byte-compilation) and a security hole
    when the expression can contain user input.
    """
    expr_indices = arg_indices_for_role(cmd_name, args, ArgRole.EXPR)
    if not expr_indices:
        return []

    diagnostics: list[Diagnostic] = []
    for idx in sorted(expr_indices):
        if idx >= len(args) or idx >= len(arg_tokens):
            continue

        tok = arg_tokens[idx]
        range_for_diag = range_from_token(tok)
        text = args[idx]

        # expr accepts expression text as its remaining argument words.
        # Highlight and fix the full expression span, not only the first word.
        if cmd_name == "expr" and arg_tokens:
            range_for_diag = range_from_tokens(arg_tokens)
            start = range_for_diag.start.offset
            end = range_for_diag.end.offset
            if 0 <= start <= end < len(source):
                text = source[start : end + 1]
            else:
                text = " ".join(args)
        else:
            # For non-expr command expression arguments (if/while/for), use
            # the exact source slice so substitutions like "$var" are
            # preserved in auto-fixes.
            start = range_for_diag.start.offset
            end = range_for_diag.end.offset
            if 0 <= start <= end < len(source):
                text = source[start : end + 1]

        # A braced argument is safe
        if _first_token_is_braced(tok):
            continue

        # A simple numeric/boolean literal expression is safe unbraced.
        stripped = text.strip()
        if cmd_name == "expr":
            if _is_safe_literal_expr(stripped):
                continue
        elif _is_safe_literal(stripped):
            continue

        # Build the fix: wrap in braces. For quoted expr arguments,
        # drop the outer quotes when bracing (expr "$a == $b" -> {$a == $b}).
        fix_text = text
        if cmd_name == "expr":
            stripped = text.strip()
            if len(stripped) >= 2 and stripped[0] == '"' and stripped[-1] == '"':
                fix_text = stripped[1:-1]

        fix = CodeFix(
            range=range_for_diag,
            new_text="{" + fix_text + "}",
            description="Wrap expression in braces",
        )

        if cmd_name == "expr":
            has_substitution = (
                "$" in text
                or "[" in text
                or any(t.type in (TokenType.VAR, TokenType.CMD) for t in arg_tokens)
            )
        else:
            has_substitution = (
                "$" in text or "[" in text or tok.type in (TokenType.VAR, TokenType.CMD)
            )

        if cmd_name == "expr":
            msg = (
                "Expression is not braced: may cause double substitution "
                "and prevents byte-compilation. Use expr {...} instead."
            )
        else:
            msg = (
                f"Expression argument to '{cmd_name}' is not braced: "
                f"may cause double substitution. Use braces: {{{text}}}"
            )

        diagnostics.append(
            Diagnostic(
                range=range_for_diag,
                message=msg,
                severity=Severity.ERROR if has_substitution else Severity.WARNING,
                code="W100",
                fixes=(fix,),
            )
        )

    return diagnostics


# W308: subst without -nocommands


def check_subst_nocommands(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W308: Warn when subst is used without -nocommands.

    Without -nocommands, any ``[cmd]`` sequence in the template string
    will be executed as a Tcl command.  This is the primary code-execution
    risk of subst -- even a literal template can be dangerous if it
    incorporates data that might contain brackets.

    If -nocommands is present, the check is satisfied.
    """
    if cmd_name != "subst":
        return []
    if not args or not arg_tokens:
        return []

    template_idx, nocommands, _, _ = _parse_subst_flags(args)

    if nocommands:
        return []

    if template_idx is None or template_idx >= len(arg_tokens):
        return []

    tok = arg_tokens[template_idx]
    text = args[template_idx]

    # If template is a variable, W102 already covers this more strongly.
    if tok.type == TokenType.VAR:
        return []

    # For literal/braced templates, check if content has brackets
    # (either literal or via command substitution tokens).
    has_bracket = "[" in text or tok.type == TokenType.CMD
    if not has_bracket:
        # Template has no command substitutions -- -nocommands wouldn't
        # change behaviour, so no point warning.
        return []

    return [
        Diagnostic(
            range=range_from_token(all_tokens[0]),
            message=(
                "subst without -nocommands will execute [cmd] sequences "
                "in the template. Add -nocommands if command substitution "
                "is not intended."
            ),
            severity=Severity.HINT,
            code="W308",
        )
    ]


# W105: Unbraced code block


def check_unbraced_body(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W105: Warn when a code-block argument is not braced.

    Commands like ``if``, ``while``, ``foreach`` expect braced body
    arguments.  Unbraced bodies risk double substitution when they
    contain ``$var`` or ``[cmd]``.
    """
    body_indices = arg_indices_for_role(cmd_name, args, ArgRole.BODY)
    if not body_indices:
        return []

    diagnostics: list[Diagnostic] = []
    for idx in sorted(body_indices):
        if idx >= len(args) or idx >= len(arg_tokens):
            continue

        tok = arg_tokens[idx]
        text = args[idx]

        if _first_token_is_braced(tok):
            continue

        # Skip if body looks like a single bare word (e.g. a proc name,
        # which some commands accept as an alternative form).
        if " " not in text.strip() and not _has_substitution(text, tok):
            continue

        dangerous = _has_substitution(text, tok)

        fix = CodeFix(
            range=range_from_token(tok),
            new_text="{" + text + "}",
            description="Wrap code block in braces",
        )

        if dangerous:
            msg = (
                f"Code block argument to '{cmd_name}' is not braced and "
                "contains substitutions \u2014 risk of double substitution. "
                "Use braces: { \u2026 }"
            )
        else:
            msg = (
                f"Code block argument to '{cmd_name}' should be braced "
                "for clarity and to prevent accidental substitution. "
                "Use braces: { \u2026 }"
            )

        diagnostics.append(
            Diagnostic(
                range=range_from_token(tok),
                message=msg,
                severity=Severity.ERROR if dangerous else Severity.WARNING,
                code="W105",
                fixes=(fix,),
            )
        )

    return diagnostics


# W106: Dangerous unbraced switch body


def check_unbraced_switch_body(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W106: Warn when switch uses unbraced pattern/body pairs.

    ``switch`` with unbraced body arguments is dangerous because patterns
    and actions undergo an extra round of substitution.  This is
    especially hazardous with ``-regexp`` mode.
    """
    if cmd_name != "switch":
        return []
    if not args or not arg_tokens:
        return []

    # Parse option flags to find the subject string and detect -regexp.
    i = 0
    has_regexp = False
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "-regexp":
            has_regexp = True
        if args[i] == "--":
            i += 1
            break
        i += 1

    # Skip subject string.
    if i >= len(args):
        return []
    i += 1  # subject

    if i >= len(args):
        return []

    # If single remaining arg (braced-list form), the body check (W105)
    # or expression bracing handles it; this check targets the
    # alternating pattern/body form.
    if i == len(args) - 1:
        # Single braced block -- check if it's braced.
        tok = arg_tokens[i]
        if _first_token_is_braced(tok):
            return []
        text = args[i]
        dangerous = _has_substitution(text, tok)
        msg = (
            "switch body is not braced \u2014 "
            + ("contains substitutions that risk code injection. " if dangerous else "")
            + "Use braces: switch \u2026 { pattern { body } \u2026 }"
        )
        return [
            Diagnostic(
                range=range_from_token(tok),
                message=msg,
                severity=Severity.ERROR if dangerous else Severity.WARNING,
                code="W106",
            )
        ]

    # Alternating pattern/body pairs.
    diagnostics: list[Diagnostic] = []
    while i + 1 < len(args):
        body_idx = i + 1
        if body_idx < len(arg_tokens):
            tok = arg_tokens[body_idx]
            text = args[body_idx]
            if not _first_token_is_braced(tok) and text != "-":
                dangerous = _has_substitution(text, tok) or has_regexp
                if has_regexp:
                    msg = (
                        "switch -regexp body is not braced \u2014 "
                        "patterns and actions undergo extra substitution, "
                        "risking code injection. Use braces: { \u2026 }"
                    )
                elif _has_substitution(text, tok):
                    msg = (
                        "switch body is not braced and contains "
                        "substitutions \u2014 risk of code injection. "
                        "Use braces: { \u2026 }"
                    )
                else:
                    msg = (
                        "switch body should be braced to prevent "
                        "accidental substitution. Use braces: { \u2026 }"
                    )
                diagnostics.append(
                    Diagnostic(
                        range=range_from_token(tok),
                        message=msg,
                        severity=Severity.ERROR if dangerous else Severity.WARNING,
                        code="W106",
                    )
                )
        i += 2

    return diagnostics


# W104: String concatenation to build list (append vs lappend)


def check_string_list_confusion(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W104: Warn when append is used in patterns suggesting list-building.

    Using [append] with spaces to build list-like strings is fragile.
    If data contains special characters the result won't be a valid Tcl list.
    Use [lappend] for list construction.
    """
    if cmd_name != "append":
        return []
    if len(args) < 2 or len(arg_tokens) < 2:
        return []

    # Check if any appended value starts/ends with space (common list-building pattern)
    for i in range(1, len(args)):
        text = args[i]
        if text.startswith(" ") or text.endswith(" "):
            tok = arg_tokens[i] if i < len(arg_tokens) else arg_tokens[0]
            return [
                Diagnostic(
                    range=range_from_token(tok),
                    message=(
                        "append with space-separated values looks like list "
                        "construction. Use [lappend] instead to safely handle "
                        "values containing spaces, braces, or backslashes."
                    ),
                    severity=Severity.HINT,
                    code="W104",
                )
            ]

    return []


# W105: namespace eval missing 'variable' declaration


def check_namespace_var_declaration(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W105: Hint when a proc inside namespace eval uses a namespace variable
    without a 'variable' declaration.

    This is checked at a higher level in the analyser (scope-aware), so this
    is a no-op placeholder -- the actual check is in check_scope_issues.
    """
    return []


# W200: exec result not captured


def check_exec_not_captured(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W200: Hint when exec result is not captured.

    exec returns the stdout of the command.  If the result is not used
    (not in a [set x [exec ...]] or similar), the output is silently lost.
    This check only fires when exec is a top-level command, not inside
    command substitution.
    """
    # This is hard to check at the token level -- we'd need to know if the
    # exec is inside a [set x [...]] context.  For now we skip this.
    return []


# W201: Possible path concatenation instead of file join

_PATH_CONCAT_RE = re.compile(r"[/\\]")


def check_path_concatenation(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W201: Warn when set uses string concatenation with path separators.

    Manual path construction with "/" or "\\" is fragile across platforms.
    Use [file join] instead.
    """
    if cmd_name != "set":
        return []
    if len(args) < 2 or len(arg_tokens) < 2:
        return []

    # Check if the value contains both path separators and a variable.
    val_text = args[1]
    reconstructed = _reconstruct_word_from_tokens(all_tokens[2:]) if len(all_tokens) > 2 else ""
    if reconstructed:
        val_text = reconstructed
    val_tok = arg_tokens[1]

    has_slash = "/" in val_text or "\\" in val_text
    has_var = "$" in val_text or any(tok.type is TokenType.VAR for tok in all_tokens[2:])

    if has_slash and has_var:
        fixes: tuple[CodeFix, ...] = ()
        replacement = _build_file_join_fix(val_text)
        if replacement is not None:
            fixes = (
                CodeFix(
                    range=range_from_token(val_tok),
                    new_text=replacement,
                    description="Rewrite as [file join ...]",
                ),
            )
        return [
            Diagnostic(
                range=range_from_token(val_tok),
                message=(
                    "Possible manual path concatenation. Use [file join] "
                    "for portable path construction."
                ),
                severity=Severity.HINT,
                code="W201",
                fixes=fixes,
            )
        ]

    return []


# W304: Missing option terminator (--) on option-bearing commands


def check_missing_option_terminator(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W304: Warn when an option-bearing command takes dynamic input without '--'.

    For commands that parse options, a user-controlled value beginning with '-'
    can be interpreted as an option (option injection).  The conventional Tcl
    mitigation is to insert '--' before untrusted positional values.
    """
    if not args or not arg_tokens:
        return []

    profile, subcommand = _resolve_option_terminator_profile(cmd_name, args)
    if profile is None:
        return []

    positional_idx = _first_positional_without_terminator(args, profile)
    if positional_idx is None or positional_idx >= len(arg_tokens):
        return []

    tok = arg_tokens[positional_idx]
    text = args[positional_idx]

    # Focus on option-injection risks (dynamic value), plus obvious literal '-'
    is_dynamic = tok.type in (TokenType.VAR, TokenType.CMD)
    looks_like_option = text.startswith("-")
    should_warn = profile.warn_without_terminator or is_dynamic or looks_like_option
    if not should_warn:
        return []

    severity = Severity.WARNING
    origin_diag: Diagnostic | None = None
    command_label = cmd_name if subcommand is None else f"{cmd_name} {subcommand}"
    if is_dynamic and tok.type is TokenType.VAR and not looks_like_option:
        resolved = _last_literal_set_value_for_var(
            source,
            tok.text,
            before_offset=tok.start.offset,
        )
        if resolved is not None and not resolved[0].startswith("-"):
            resolved_text, resolved_range = resolved
            severity = Severity.INFO
            message = (
                f"'{command_label}' parses leading '-' as options. This value "
                f"is reported at INFO because '{tok.text}' currently resolves "
                f"to static literal '{resolved_text}'. Keep '--' to guard "
                "against future option-injection regressions if the variable changes."
            )
            origin_diag = Diagnostic(
                range=resolved_range,
                message=(
                    f"'{tok.text}' is currently assigned static literal "
                    f"'{resolved_text}' here; this is why the switch-site "
                    "diagnostic is INFO."
                ),
                severity=Severity.INFO,
                code="W304",
            )
        elif resolved is not None and resolved[0].startswith("-"):
            resolved_text, _ = resolved
            message = (
                f"'{command_label}' parses leading '-' as options. This value "
                f"currently resolves to '{resolved_text}', so add '--' to force data "
                "parsing."
            )
        else:
            message = (
                f"'{command_label}' parses leading '-' as options. Insert '--' "
                "before substituted input to reduce option-injection risk."
            )
    elif profile.warn_without_terminator and not is_dynamic and not looks_like_option:
        message = (
            f"'{command_label}' parses leading '-' as options. Insert '--' "
            "before the first positional argument for predictable parsing."
        )
    elif is_dynamic:
        message = (
            f"'{command_label}' parses leading '-' as options. Insert '--' "
            "before substituted input to reduce option-injection risk."
        )
    else:
        message = (
            f"'{command_label}' argument starts with '-'. Add '--' before this "
            "value so it is treated as data, not an option."
        )

    # Build the fix range and replacement text from source.  For CMD tokens
    # the lexer range covers ``[inner`` but not the closing ``]``, so we
    # extend the slice by one character to include it.
    fix: CodeFix | None = None
    diag_end = tok.end

    if tok.type is TokenType.CMD:
        close = tok.end.offset + 1
        if close < len(source) and source[close] == "]":
            fix_end = SourcePosition(
                line=tok.end.line,
                character=tok.end.character + 1,
                offset=close,
            )
            diag_end = fix_end
            tok_source = source[tok.start.offset : fix_end.offset + 1]
            fix = CodeFix(
                range=Range(start=tok.start, end=fix_end),
                new_text="-- " + tok_source,
                description="Insert '--' option terminator",
            )
        else:
            # Unterminated CMD -- tighten the diagnostic range to just
            # the content before the first '{' (same as E201 recovery).
            # No CodeFix here; E201's fix handles the bracket.
            brace_idx = tok.text.find("{")
            if brace_idx > 0:
                trunc = tok.text[:brace_idx].rstrip()
                diag_end = _pos_in_cmd_text(tok, max(len(trunc) - 1, 0))
            else:
                diag_end = tok.start
    else:
        tok_source = source[tok.start.offset : tok.end.offset + 1]
        fix = CodeFix(
            range=Range(start=tok.start, end=tok.end),
            new_text="-- " + tok_source,
            description="Insert '--' option terminator",
        )

    result = [
        Diagnostic(
            range=Range(start=tok.start, end=diag_end),
            message=message,
            severity=severity,
            code="W304",
            fixes=(fix,) if fix is not None else (),
        )
    ]
    if origin_diag is not None:
        result.append(origin_diag)
    return result


# W110: if/while/for expr with == on strings (should use eq/ne)


def _find_string_eq_ne(node: ExprNode) -> list[BinOp]:
    """Walk *node* and collect ``==``/``!=`` ops where at least one operand is
    a string literal (``ExprString``).

    Comparisons between two variables (``$x == $y``) are *not* collected
    because the variables may hold integer values, making ``==`` correct.
    Only when a quoted/braced string literal appears on at least one side
    can we be confident the user intended a string comparison.
    """
    found: list[BinOp] = []

    match node:
        case ExprBinary(op=op, left=left, right=right):
            found.extend(_find_string_eq_ne(left))
            found.extend(_find_string_eq_ne(right))
            if op in (BinOp.EQ, BinOp.NE):
                if isinstance(left, ExprString) or isinstance(right, ExprString):
                    found.append(op)
        case ExprUnary(operand=operand):
            found.extend(_find_string_eq_ne(operand))
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            found.extend(_find_string_eq_ne(cond))
            found.extend(_find_string_eq_ne(tb))
            found.extend(_find_string_eq_ne(fb))
        case ExprCall(args=args):
            for arg in args:
                found.extend(_find_string_eq_ne(arg))

    return found


def _count_eq_ne_ops(node: ExprNode) -> int:
    """Count total ``==``/``!=`` operators in the expression tree."""
    match node:
        case ExprBinary(op=op, left=left, right=right):
            count = _count_eq_ne_ops(left) + _count_eq_ne_ops(right)
            if op in (BinOp.EQ, BinOp.NE):
                count += 1
            return count
        case ExprUnary(operand=operand):
            return _count_eq_ne_ops(operand)
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            return _count_eq_ne_ops(cond) + _count_eq_ne_ops(tb) + _count_eq_ne_ops(fb)
        case ExprCall(args=args):
            return sum(_count_eq_ne_ops(arg) for arg in args)
        case _:
            return 0


def check_string_compare_in_expr(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W110: Hint when == or != is used in an expression that compares strings.

    In Tcl's expr, == and != perform numeric comparison when both operands
    look numeric, and string comparison otherwise.  This implicit behaviour
    can cause bugs.  Use 'eq' / 'ne' for explicit string comparison, or
    [string equal] / [string compare].

    Fires when at least one operand of the comparison is a string literal
    (``ExprString``, e.g. ``"foo"``, ``"1"``, ``"true"``).  Comparisons
    between variables that may hold integers (``$x == $y``) are left
    alone — even when the expression text happens to contain double-quote
    characters from outer quoting.
    """
    expr_indices = arg_indices_for_role(cmd_name, args, ArgRole.EXPR)
    if not expr_indices:
        return []

    diagnostics: list[Diagnostic] = []
    for idx in sorted(expr_indices):
        if idx >= len(args) or idx >= len(arg_tokens):
            continue

        tok = arg_tokens[idx]
        range_for_fix = range_from_token(tok)
        text = args[idx]
        if cmd_name == "expr" and arg_tokens:
            range_for_fix = range_from_tokens(arg_tokens)
            start = range_for_fix.start.offset
            end = range_for_fix.end.offset
            if 0 <= start <= end < len(source):
                text = source[start : end + 1]
            else:
                text = " ".join(args)

        # Quick bail-out: no equality operator at all.
        if "==" not in text and "!=" not in text:
            continue

        # Parse the expression from the argument value (without quoting
        # delimiters like braces) so parse_expr sees pure expression text.
        # Keep `text` for the code-fix replacement, which needs to match
        # the source span.
        expr_text = args[idx] if cmd_name == "expr" else text
        if cmd_name == "expr" and len(args) > 1:
            expr_text = " ".join(args)
        parsed = parse_expr(expr_text.strip(), dialect=active_dialect())
        if isinstance(parsed, ExprRaw):
            continue

        matched_ops = _find_string_eq_ne(parsed)
        if not matched_ops:
            continue

        # Pick the first matched operator for the message.
        first_op = matched_ops[0]
        op = "==" if first_op is BinOp.EQ else "!="
        replacement = "eq" if first_op is BinOp.EQ else "ne"

        # Only offer the regex-based code fix when every ==/ != in the
        # expression has a string-literal operand; otherwise the blanket
        # rewrite would incorrectly change non-string comparisons too.
        fixes: tuple[CodeFix, ...] = ()
        total_eq_ne = _count_eq_ne_ops(parsed)
        if len(matched_ops) >= total_eq_ne:
            fixed = _rewrite_string_compare_ops(text)
            if fixed != text:
                fixes = (
                    CodeFix(
                        range=range_for_fix,
                        new_text=fixed,
                        description=f"Use '{replacement}' for string comparison",
                    ),
                )
        diagnostics.append(
            Diagnostic(
                range=range_for_fix,
                message=(
                    f"Use '{replacement}' instead of '{op}' for string "
                    f"comparison in expressions to avoid ambiguous "
                    f"numeric/string coercion."
                ),
                severity=Severity.HINT,
                code="W110",
                fixes=fixes,
            )
        )

    return diagnostics


# W200: binary format signed/unsigned modifier requires Tcl 8.5+

# Integer specifiers that accept u/s modifiers in binary format/scan.
_BINARY_INT_SPECIFIERS = frozenset({"c", "s", "S", "i", "I", "n", "t", "w", "W", "m", "r", "R"})


def check_binary_format_modifiers(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W200: Warn when signed/unsigned modifiers are used under Tcl 8.4.

    Tcl 8.5 added ``u`` and ``s`` suffix modifiers to integer format
    specifiers in ``binary format`` and ``binary scan``.  These are not
    available in Tcl 8.4 or dialects based on it (F5 iRules / iApps).
    """
    if cmd_name != "binary" or not args:
        return []
    sub = args[0]
    if sub == "format" and len(args) >= 2:
        fmt_idx = 1
    elif sub == "scan" and len(args) >= 3:
        fmt_idx = 2
    else:
        return []

    if active_dialect() not in ("tcl8.4", "f5-irules", "f5-iapps"):
        return []

    fmt_text = args[fmt_idx]
    fmt_tok = arg_tokens[fmt_idx]
    diagnostics: list[Diagnostic] = []
    i = 0
    while i < len(fmt_text):
        ch = fmt_text[i]
        if ch in " \t\r\n":
            i += 1
            continue
        # skip count digits
        while i < len(fmt_text) and fmt_text[i].isdigit():
            i += 1
        if i >= len(fmt_text):
            break
        spec = fmt_text[i]
        i += 1
        if spec in _BINARY_INT_SPECIFIERS and i < len(fmt_text) and fmt_text[i] in ("u", "s"):
            modifier = fmt_text[i]
            diagnostics.append(
                Diagnostic(
                    range=range_from_token(fmt_tok),
                    message=(
                        f"signed/unsigned modifier '{modifier}' on binary "
                        f"format specifier requires Tcl 8.5+"
                    ),
                    severity=Severity.WARNING,
                    code="W200",
                )
            )
            i += 1
        # skip repeat count after specifier
        if i < len(fmt_text) and fmt_text[i] == "*":
            i += 1

    return diagnostics


# W212: Variable substitution where a variable *name* is expected

_NameResolver = Callable[[list[str]], list[int]]

_NAME_ARG_INDICES: dict[str, _NameResolver] = {
    "set": _first_arg_name,
    "incr": _first_arg_name,
    "append": _first_arg_name,
    "lappend": _first_arg_name,
    "unset": _unset_name_args,
    "info": _info_exists_arg,
    "upvar": _upvar_local_name_args,
}


def check_name_vs_value(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W212: Warn when a variable *name* argument uses ``$``-substitution."""
    resolver = _NAME_ARG_INDICES.get(cmd_name)
    if resolver is None:
        return []
    indices = resolver(args)
    if not indices:
        return []
    diagnostics: list[Diagnostic] = []
    for idx in indices:
        if idx >= len(arg_tokens):
            continue
        if arg_tokens[idx].type is not TokenType.VAR:
            continue
        var_name = args[idx]
        # Build display command for the message.
        if cmd_name == "info" and args:
            display_cmd = f"info {args[0]}"
        else:
            display_cmd = cmd_name
        diagnostics.append(
            Diagnostic(
                range=range_from_token(arg_tokens[idx]),
                message=(
                    f"'{display_cmd}' expects a variable name, "
                    f"got substitution (${var_name}). "
                    f"Did you mean '{var_name}'?"
                ),
                severity=Severity.WARNING,
                code="W212",
            )
        )
    return diagnostics


# W108: Non-ASCII token content

# Characters that can be auto-fixed to their ASCII equivalents.
# Covers common copy-paste artifacts from Slack, Teams, Outlook, Word,
# macOS auto-correct, and web browsers.
_AUTO_FIX_MAP: dict[str, str] = {
    # ── Smart / curly quotes (Slack, Teams, Outlook, Word all do this) ──
    "\u201c": '"',  # " left double quotation mark
    "\u201d": '"',  # " right double quotation mark
    "\u201e": '"',  # „ double low-9 quotation mark
    "\u2018": "'",  # ' left single quotation mark
    "\u2019": "'",  # ' right single quotation mark  (also Slack apostrophe)
    "\u201a": "'",  # ‚ single low-9 quotation mark
    "\u00ab": '"',  # « left-pointing double angle quotation mark
    "\u00bb": '"',  # » right-pointing double angle quotation mark
    "\u2039": "'",  # ‹ single left-pointing angle quotation mark
    "\u203a": "'",  # › single right-pointing angle quotation mark
    "\u0060": "'",  # ` grave accent (Outlook sometimes substitutes)
    "\u00b4": "'",  # ´ acute accent
    "\u02bc": "'",  # ʼ modifier letter apostrophe (macOS auto-correct)
    # ── Spaces (Outlook, Word, web copy-paste) ──
    "\u00a0": " ",  # non-breaking space (Outlook's favourite)
    "\u2002": " ",  # en space
    "\u2003": " ",  # em space
    "\u2009": " ",  # thin space
    "\u200a": " ",  # hair space
    "\u202f": " ",  # narrow no-break space
    "\u205f": " ",  # medium mathematical space
    "\u200b": "",  # zero-width space (Slack, web copy-paste)
    "\u200c": "",  # zero-width non-joiner
    "\u200d": "",  # zero-width joiner
    "\ufeff": "",  # BOM / zero-width no-break space
    # ── Dashes (Slack, Word, Outlook auto-format) ──
    "\u2013": "-",  # – en dash
    "\u2014": "--",  # — em dash
    "\u2015": "--",  # ― horizontal bar
    "\u2012": "-",  # ‒ figure dash
    "\u2010": "-",  # ‐ hyphen (Unicode hyphen, not ASCII)
    "\u2011": "-",  # ‑ non-breaking hyphen
    "\u2212": "-",  # − minus sign
    "\ufe63": "-",  # ﹣ small hyphen-minus
    "\uff0d": "-",  # － fullwidth hyphen-minus
    # ── Arrows ──
    "\u2190": "<-",  # ← leftwards arrow
    "\u2192": "->",  # → rightwards arrow
    "\u21d0": "<=",  # ⇐ leftwards double arrow
    "\u21d2": "=>",  # ⇒ rightwards double arrow
    # ── Ellipsis (macOS auto-correct, Slack, Word) ──
    "\u2026": "...",  # … horizontal ellipsis
    # ── Comparison / math operators ──
    "\u2264": "<=",  # ≤ less-than or equal to
    "\u2265": ">=",  # ≥ greater-than or equal to
    "\u2260": "!=",  # ≠ not equal to
    "\u00d7": "*",  # × multiplication sign
    "\u00f7": "/",  # ÷ division sign
    # ── Fullwidth ASCII variants (CJK keyboards, Teams) ──
    "\uff08": "(",  # （ fullwidth left parenthesis
    "\uff09": ")",  # ） fullwidth right parenthesis
    "\uff3b": "[",  # ［ fullwidth left square bracket
    "\uff3d": "]",  # ］ fullwidth right square bracket
    "\uff5b": "{",  # ｛ fullwidth left curly bracket
    "\uff5d": "}",  # ｝ fullwidth right curly bracket
    "\uff1b": ";",  # ； fullwidth semicolon
    "\uff1a": ":",  # ： fullwidth colon
    "\uff0c": ",",  # ， fullwidth comma
    "\uff0e": ".",  # ． fullwidth full stop
    "\uff01": "!",  # ！ fullwidth exclamation mark
    "\uff1f": "?",  # ？ fullwidth question mark
    "\uff04": "$",  # ＄ fullwidth dollar sign
    "\uff20": "@",  # ＠ fullwidth commercial at
    "\uff03": "#",  # ＃ fullwidth number sign
    "\uff05": "%",  # ％ fullwidth percent sign
    "\uff06": "&",  # ＆ fullwidth ampersand
    "\uff0a": "*",  # ＊ fullwidth asterisk
    "\uff0b": "+",  # ＋ fullwidth plus sign
    "\uff1c": "<",  # ＜ fullwidth less-than sign
    "\uff1d": "=",  # ＝ fullwidth equals sign
    "\uff1e": ">",  # ＞ fullwidth greater-than sign
    "\uff3c": "\\",  # ＼ fullwidth reverse solidus
    "\uff0f": "/",  # ／ fullwidth solidus
    "\uff5c": "|",  # ｜ fullwidth vertical line
    "\uff5e": "~",  # ～ fullwidth tilde
    # ── Misc punctuation ──
    "\u2022": "*",  # • bullet
    "\u2027": "-",  # ‧ hyphenation point
    "\u00b7": ".",  # · middle dot
    "\u2032": "'",  # ′ prime
    "\u2033": '"',  # ″ double prime
    "\u2044": "/",  # ⁄ fraction slash
    "\u2016": "||",  # ‖ double vertical line
    # ── Emoji that Slack/Teams auto-convert from ASCII sequences ──
    # Slack converts these shortcodes/ASCII to emoji; provide the
    # most likely ASCII the user originally typed.
    "\U0001f60a": ":)",  # 😊 smiling face with smiling eyes  (:) in Slack)
    "\U0001f642": ":)",  # 🙂 slightly smiling face
    "\U0001f600": ":D",  # 😀 grinning face
    "\U0001f603": ":D",  # 😃 grinning face with big eyes
    "\U0001f604": ":D",  # 😄 grinning face with smiling eyes
    "\U0001f601": ":D",  # 😁 beaming face with smiling eyes
    "\U0001f61e": ":(",  # 😞 disappointed face
    "\U0001f641": ":(",  # 🙁 slightly frowning face
    "\U0001f622": ":'(",  # 😢 crying face
    "\U0001f609": ";)",  # 😉 winking face
    "\U0001f60d": "<3",  # 😍 smiling face with heart-eyes
    "\u2764": "<3",  # ❤ red heart (also ❤️)
    "\U0001f44d": "+1",  # 👍 thumbs up
    "\U0001f44e": "-1",  # 👎 thumbs down
    "\U0001f61b": ":P",  # 😛 face with tongue
    "\U0001f61c": ";P",  # 😜 winking face with tongue
    "\U0001f610": ":|",  # 😐 neutral face
    "\U0001f914": ":?",  # 🤔 thinking face
    "\U0001f631": ":O",  # 😱 face screaming in fear
    "\U0001f62e": ":O",  # 😮 face with open mouth
    "\U0001f913": "8)",  # 🤓 nerd face
    "\U0001f60e": "B)",  # 😎 smiling face with sunglasses
    "\U0001f911": ":$",  # 🤑 money-mouth face  (:$ in Slack)
    "\U0001f618": ":*",  # 😘 face blowing a kiss  (:* in Slack)
    "\U0001f4af": "100",  # 💯 hundred points
    "\u2705": "[x]",  # ✅ check mark
    "\u274c": "[x]",  # ❌ cross mark
    "\u2714": "[x]",  # ✔ check mark (heavy)
    "\u2716": "[x]",  # ✖ heavy multiplication X
    "\U0001f525": "*",  # 🔥 fire
    "\U0001f389": "!",  # 🎉 party popper
    "\U0001f680": "=>",  # 🚀 rocket
}


def check_non_ascii(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W108: Warn when tokens contain non-standard ASCII characters.

    Characters outside the printable ASCII range (0x20-0x7E) plus
    standard whitespace can indicate encoding issues or copy-paste
    artifacts.  For known copy-paste characters (smart quotes,
    non-breaking spaces) a quick-fix CodeFix is attached.

    Each offending character gets its own diagnostic so the editor
    highlights the exact position rather than the whole (potentially
    multi-line) token.
    """
    diagnostics: list[Diagnostic] = []
    seen_offsets: set[int] = set()

    for tok, text in zip(arg_tokens, args):
        if tok.start.offset in seen_offsets:
            continue
        # Skip multi-line brace-quoted body tokens — the individual
        # commands inside will be checked when the body is recursed into,
        # which avoids duplicate diagnostics and lets each one pinpoint
        # the exact location within the correct inner token.
        if "\n" in text and tok.type is TokenType.ESC:
            continue

        bad_positions: list[int] = []
        for i, ch in enumerate(text):
            code_point = ord(ch)
            if 0x20 <= code_point <= 0x7E:
                continue
            if ch in ("\t", "\n", "\r"):
                continue
            bad_positions.append(i)

        if not bad_positions:
            continue

        seen_offsets.add(tok.start.offset)

        for bad_idx in bad_positions:
            ch = text[bad_idx]
            char_start = position_from_relative(
                text,
                bad_idx,
                base_line=tok.start.line,
                base_col=tok.start.character,
                base_offset=tok.start.offset,
            )
            char_end = position_from_relative(
                text,
                bad_idx + 1,
                base_line=tok.start.line,
                base_col=tok.start.character,
                base_offset=tok.start.offset,
            )
            char_range = Range(start=char_start, end=char_end)

            fixes: tuple[CodeFix, ...] = ()
            if ch in _AUTO_FIX_MAP:
                fixes = (
                    CodeFix(
                        range=char_range,
                        new_text=_AUTO_FIX_MAP[ch],
                        description="Replace with ASCII equivalent",
                    ),
                )

            char_desc = "U+{:04X}".format(ord(ch))
            diagnostics.append(
                Diagnostic(
                    range=char_range,
                    message=(
                        "Non-ASCII character {} '{}' — outside the standard "
                        "ASCII printable/whitespace set".format(char_desc, ch)
                    ),
                    severity=Severity.WARNING,
                    code="W108",
                    fixes=fixes,
                )
            )

    return diagnostics


# IRULE5003: while {$var != 0} with decrement

_LOOP_NE_ZERO_RE = re.compile(
    r"""
    (?:
        \$\{?(\w+)\}? \s* (?:!=|ne) \s* 0   # $var != 0  or  $var ne 0
    |
        0 \s* (?:!=|ne) \s* \$\{?(\w+)\}?    # 0 != $var  or  0 ne $var
    )
    """,
    re.VERBOSE,
)
_INCR_DECREMENT_RE = re.compile(r"\bincr\s+(\w+)\s+(-\d+)\b")


def check_loop_bound_inequality(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """IRULE5003: ``while {$var != 0} { incr var -1 }`` can miss zero.

    If the counter starts negative, it decrements past zero and loops ~2^63
    times.  Suggest ``$var > 0`` instead.
    """
    if cmd_name != "while" or len(args) < 2:
        return []

    condition = args[0]
    body = args[1]

    m = _LOOP_NE_ZERO_RE.search(condition)
    if m is None:
        return []
    var_name = m.group(1) or m.group(2)

    # Check if the body decrements the same variable.
    for dm in _INCR_DECREMENT_RE.finditer(body):
        if dm.group(1) == var_name:
            return [
                Diagnostic(
                    range=range_from_token(arg_tokens[0]),
                    message=(
                        f"Loop condition '${var_name} != 0' can miss zero if "
                        f"decremented past it. Consider '${var_name} > 0'."
                    ),
                    severity=Severity.HINT,
                    code="IRULE5003",
                )
            ]
    return []


# W114: Redundant nested [expr]

_NESTED_EXPR_RE = re.compile(r"\[\s*expr\s")


def check_redundant_expr(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W114: Warn on redundant nested ``[expr ...]`` inside an expression context.

    Commands like ``expr``, ``if``, ``while``, and ``for`` already evaluate
    their argument as an expression, so ``expr {[expr {$x + 1}]}`` is
    redundant and indicates a misunderstanding of Tcl's evaluation model.
    """
    expr_indices = arg_indices_for_role(cmd_name, args, ArgRole.EXPR)
    if not expr_indices:
        return []

    diagnostics: list[Diagnostic] = []
    for idx in sorted(expr_indices):
        if idx >= len(args) or idx >= len(arg_tokens):
            continue
        text = args[idx]
        m = _NESTED_EXPR_RE.search(text)
        if m is None:
            continue
        tok = arg_tokens[idx]
        start = _pos_in_cmd_text(tok, m.start())
        # Find the matching closing ']' for the range end.
        depth = 1
        end_idx = m.end()
        while end_idx < len(text) and depth > 0:
            if text[end_idx] == "[":
                depth += 1
            elif text[end_idx] == "]":
                depth -= 1
            end_idx += 1
        end = _pos_in_cmd_text(tok, end_idx - 1)
        diagnostics.append(
            Diagnostic(
                range=Range(start=start, end=end),
                message="Redundant nested [expr] — already in expression context",
                severity=Severity.WARNING,
                code="W114",
            )
        )
    return diagnostics


# W311: Unsafe channel encoding mismatch


def check_encoding_mismatch(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W311: Warn when fconfigure/chan configure sets binary encoding alongside text flags.

    Setting ``-encoding binary`` and ``-translation`` at the same time is
    contradictory: binary mode implies no translation, and adding a
    translation mode undermines the binary guarantee.  This can silently
    corrupt multibyte characters or enable encoding-differential attacks.
    """
    if cmd_name == "fconfigure":
        opt_start = 1  # fconfigure channelId ?name value ...?
    elif cmd_name == "chan" and args and args[0] == "configure":
        opt_start = 2  # chan configure channelId ?name value ...?
    else:
        return []

    if len(args) <= opt_start:
        return []

    has_binary_encoding = False
    has_translation = False
    binary_tok: Token | None = None
    translation_tok: Token | None = None

    i = opt_start
    while i < len(args) - 1:
        opt = args[i]
        val = args[i + 1] if i + 1 < len(args) else ""
        if opt == "-encoding" and val == "binary":
            has_binary_encoding = True
            if i + 1 < len(arg_tokens):
                binary_tok = arg_tokens[i + 1]
        elif opt == "-translation" and val != "binary":
            has_translation = True
            if i + 1 < len(arg_tokens):
                translation_tok = arg_tokens[i + 1]
        i += 2

    if has_binary_encoding and has_translation:
        target_tok = translation_tok or binary_tok or arg_tokens[0]
        return [
            Diagnostic(
                range=range_from_token(target_tok),
                message=(
                    "Channel configured with -encoding binary and a non-binary "
                    "-translation. Binary encoding implies no translation; the "
                    "conflicting -translation may silently corrupt data or "
                    "enable encoding-differential attacks."
                ),
                severity=Severity.WARNING,
                code="W311",
            )
        ]

    return []


# W121: Invalid subnet mask — dotted-quad looks like a mask but bits are not contiguous

# Valid single-octet values for a subnet mask boundary.
_VALID_MASK_OCTETS = frozenset({0, 128, 192, 224, 240, 248, 252, 254, 255})

_DOTTED_QUAD_RE = re.compile(
    r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b",
)


def _is_valid_subnet_mask(a: int, b: int, c: int, d: int) -> bool:
    """Return True if the four octets form a valid contiguous subnet mask."""
    val = (a << 24) | (b << 16) | (c << 8) | d
    if val == 0:
        return True
    # A valid mask is all-1s followed by all-0s.
    # Inverting gives all-0s followed by all-1s, i.e. (inverted + 1) is a power of 2.
    inverted = val ^ 0xFFFFFFFF
    return (inverted & (inverted + 1)) == 0


def _looks_like_subnet_mask(a: int, b: int, c: int, d: int) -> bool:
    """Heuristic: return True when a dotted-quad plausibly *intends* to be a mask.

    Criteria — at least one must hold:
    * First octet is 255 and the address is not 255.255.255.255.
    * Every octet is drawn from the valid mask-octet set and the first
      octet is >= 128 (eliminates most regular IPs).
    """
    if a == 255 and not (b == 255 and c == 255 and d == 255):
        return True
    if a >= 128 and {a, b, c, d} <= _VALID_MASK_OCTETS:
        return True
    return False


def _cidr_from_mask(a: int, b: int, c: int, d: int) -> int | None:
    """Return the CIDR prefix length for a valid contiguous mask, else None."""
    val = (a << 24) | (b << 16) | (c << 8) | d
    if val == 0:
        return 0
    inverted = val ^ 0xFFFFFFFF
    if (inverted & (inverted + 1)) != 0:
        return None
    return 32 - inverted.bit_length()


def _nearest_valid_mask(a: int, b: int, c: int, d: int) -> str | None:
    """Suggest the closest valid contiguous mask for an invalid one."""
    val = (a << 24) | (b << 16) | (c << 8) | d
    # Count the leading 1-bits.
    leading = 0
    for bit in range(31, -1, -1):
        if val & (1 << bit):
            leading += 1
        else:
            break
    if leading == 0 or leading == 32:
        return None
    candidate = (0xFFFFFFFF << (32 - leading)) & 0xFFFFFFFF
    return (
        f"{(candidate >> 24) & 0xFF}."
        f"{(candidate >> 16) & 0xFF}."
        f"{(candidate >> 8) & 0xFF}."
        f"{candidate & 0xFF}"
    )


def check_invalid_subnet_mask(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W121: Warn when a dotted-quad literal looks like a subnet mask but has non-contiguous bits.

    A valid subnet mask has contiguous leading 1-bits followed by
    contiguous trailing 0-bits (e.g. 255.255.254.0).  Values like
    255.255.255.1 or 255.0.255.0 are almost certainly mistakes.
    """
    diagnostics: list[Diagnostic] = []
    seen_offsets: set[int] = set()

    for tok, text in zip(arg_tokens, args):
        if tok.start.offset in seen_offsets:
            continue

        for m in _DOTTED_QUAD_RE.finditer(text):
            octets_str = [m.group(i) for i in range(1, 5)]
            octets = [int(o) for o in octets_str]
            if any(o > 255 for o in octets):
                continue

            a, b, c, d = octets
            if not _looks_like_subnet_mask(a, b, c, d):
                continue
            if _is_valid_subnet_mask(a, b, c, d):
                continue

            seen_offsets.add(tok.start.offset)
            quad = f"{a}.{b}.{c}.{d}"
            suggestion = _nearest_valid_mask(a, b, c, d)
            msg = (
                f"'{quad}' looks like a subnet mask but has non-contiguous bits. "
                "A valid mask must be contiguous leading 1-bits followed by 0-bits."
            )

            fixes: tuple[CodeFix, ...] = ()
            if suggestion is not None and suggestion != quad:
                msg += f" Did you mean '{suggestion}'?"
                # Build a CodeFix replacing the dotted-quad within the token.
                fix_text = text[: m.start()] + suggestion + text[m.end() :]
                # Preserve braces for STR (braced) tokens whose range
                # includes the delimiters.
                if tok.type == TokenType.STR:
                    fix_text = "{" + fix_text + "}"
                fixes = (
                    CodeFix(
                        range=range_from_token(tok),
                        new_text=fix_text,
                        description=f"Replace with valid mask {suggestion}",
                    ),
                )

            diagnostics.append(
                Diagnostic(
                    range=range_from_token(tok),
                    message=msg,
                    severity=Severity.WARNING,
                    code="W121",
                    fixes=fixes,
                )
            )
            break  # one diagnostic per token

    return diagnostics


# W122: Mistyped IPv4 address — octet out of range or leading zero

_DOTTED_QUAD_LOOSE_RE = re.compile(
    r"\b(\d{1,4})\.(\d{1,4})\.(\d{1,4})\.(\d{1,4})\b",
)


def check_mistyped_ipv4(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W122: Warn when a dotted-quad has out-of-range octets or leading zeros.

    Catches typos like ``192.168.1.256`` (octet > 255) and ambiguous
    leading zeros like ``192.168.01.1`` which may be interpreted as
    octal in some contexts.
    """
    diagnostics: list[Diagnostic] = []
    seen_offsets: set[int] = set()

    for tok, text in zip(arg_tokens, args):
        if tok.start.offset in seen_offsets:
            continue

        for m in _DOTTED_QUAD_LOOSE_RE.finditer(text):
            octets_str = [m.group(i) for i in range(1, 5)]

            # Check each octet
            for i, octet_s in enumerate(octets_str):
                val = int(octet_s)
                if val > 255:
                    seen_offsets.add(tok.start.offset)
                    diagnostics.append(
                        Diagnostic(
                            range=range_from_token(tok),
                            message=(
                                f"IPv4 octet {i + 1} ({octet_s}) exceeds 255 "
                                "— this is not a valid IP address."
                            ),
                            severity=Severity.ERROR,
                            code="W122",
                        )
                    )
                    break
                if len(octet_s) > 1 and octet_s[0] == "0" and all(c in "01234567" for c in octet_s):
                    seen_offsets.add(tok.start.offset)
                    diagnostics.append(
                        Diagnostic(
                            range=range_from_token(tok),
                            message=(
                                f"IPv4 octet {i + 1} ({octet_s}) has a leading zero "
                                "— may be interpreted as octal in some contexts."
                            ),
                            severity=Severity.WARNING,
                            code="W122",
                        )
                    )
                    break

            if tok.start.offset in seen_offsets:
                break  # one diagnostic per token

    return diagnostics
