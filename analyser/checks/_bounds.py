"""Index-bounds and loop-termination checks (W230-W242).

Diagnostics in this module validate constant indices passed to list and
string commands against known lengths, and analyse ``while`` / ``for``
loops for provable non-termination or condition-always-false patterns.

All semantics were cross-checked against Tcl 9.0.3 (built from source).

List-index semantics (Tcl 9.0.3):

* ``lindex`` -- out-of-range returns the empty string silently.
* ``lrange`` -- out-of-range clamps; result can be empty.
* ``lreplace`` -- out-of-range indices clamp to the boundary and the
  replacement is inserted at the start or end rather than replacing
  an element.  W230 calls this out explicitly (prepends/appends) when
  both indices fall outside the list.
* ``linsert`` -- out-of-range clamps to start/end; always produces a
  sensible result, so W230 does NOT apply.
* ``lset`` -- out-of-range is a *hard error* at runtime.

String-index semantics (Tcl 9.0.3):

* ``string index`` -- out-of-range returns the empty string silently.
* ``string range`` -- out-of-range clamps.
* ``string replace`` -- out-of-range is a silent no-op.
* ``string insert`` -- out-of-range clamps (negative -> front,
  too-large -> end).

The ``end-N`` suffix form collapses to a negative absolute index when
``N`` exceeds the length minus one; the same silent-empty / hard-error
distinction applies.
"""

from __future__ import annotations

import re

from shared.codes import diag
from shared.tokens import Token, TokenType

from ..semantic_model import Diagnostic, Range, Severity

# Constant-index parsing helpers
# -----------------------------------------------------------------------------

_INT_RE = re.compile(r"^-?\d+$")
_END_MINUS_RE = re.compile(r"^end\s*-\s*(\d+)$")
_END_PLUS_RE = re.compile(r"^end\s*\+\s*(\d+)$")


def _is_literal_index(s: str) -> bool:
    """Return True if *s* is a constant index expression we can evaluate."""
    s = s.strip()
    if s == "end":
        return True
    if _INT_RE.match(s):
        return True
    if _END_MINUS_RE.match(s):
        return True
    if _END_PLUS_RE.match(s):
        return True
    return False


def _resolve_index(s: str, length: int) -> int | None:
    """Resolve a constant index to an absolute offset given ``length``.

    Returns ``None`` if *s* is not a literal constant index.
    """
    s = s.strip()
    if s == "end":
        return length - 1
    m = _END_MINUS_RE.match(s)
    if m is not None:
        return length - 1 - int(m.group(1))
    m = _END_PLUS_RE.match(s)
    if m is not None:
        return length - 1 + int(m.group(1))
    if _INT_RE.match(s):
        return int(s)
    return None


def _split_list_literal(text: str) -> list[str] | None:
    """Split a static Tcl list literal into its elements, or None on failure."""
    from compiler.tcl_expr_eval import _split_tcl_list

    try:
        return _split_tcl_list(text)
    except Exception:
        return None


def _is_braced_or_esc(tok: Token) -> bool:
    """Return True if the token is a literal word (braced string or plain word)."""
    return tok.type in (TokenType.STR, TokenType.ESC)


def _has_subst(text: str, tok: Token) -> bool:
    """Return True if the word contains variable or command substitution."""
    if tok.type in (TokenType.VAR, TokenType.CMD):
        return True
    return "$" in text or "[" in text


# W230 / W231: list index out of range
# -----------------------------------------------------------------------------


def _describe_index(resolved: int, length: int) -> str:
    if resolved < 0:
        return f"resolves to {resolved} (before start of list)"
    return f"resolves to {resolved} (list has {length} element{'s' if length != 1 else ''})"


@diag(
    code="W230",
    description=(
        "Constant list index out of range — lindex/lrange/lreplace silently return empty or clamp."
    ),
    section="warning",
    ai_category="control_flow",
)
def check_list_index_out_of_range(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W230: Warn on constant-out-of-range index for silent list commands.

    Fires when the list argument is a literal (or empty) *and* at least one
    index argument is a literal that resolves outside ``[0, length)``.
    Tcl silently returns empty / clamps, which usually indicates a bug.

    ``linsert`` is deliberately excluded: its out-of-range clamp
    (``-N`` -> prepend, ``>= length`` -> append) always produces a
    meaningful result, so flagging it would be second-guessing intent.
    """
    if cmd_name not in ("lindex", "lrange", "lreplace"):
        return []
    if len(args) < 2 or len(arg_tokens) < 2:
        return []

    list_tok = arg_tokens[0]
    list_text = args[0]

    if not _is_braced_or_esc(list_tok) or _has_subst(list_text, list_tok):
        return []

    elements = _split_list_literal(list_text)
    if elements is None:
        return []
    length = len(elements)

    diagnostics: list[Diagnostic] = []

    if cmd_name == "lindex":
        # Each index must land in [0, length).  Out of range -> empty.
        for pos in range(1, len(args)):
            if pos >= len(arg_tokens):
                continue
            idx_text = args[pos]
            idx_tok = arg_tokens[pos]
            if _has_subst(idx_text, idx_tok) or not _is_literal_index(idx_text):
                continue
            resolved = _resolve_index(idx_text, length)
            if resolved is None or 0 <= resolved < length:
                continue
            diagnostics.append(
                Diagnostic(
                    range=Range(start=idx_tok.start, end=idx_tok.end),
                    message=(
                        f"Index '{idx_text}' {_describe_index(resolved, length)}; "
                        f"lindex silently returns empty string."
                    ),
                    severity=Severity.WARNING,
                    code="W230",
                )
            )
        return diagnostics

    # lrange / lreplace share a (first, last) pair; only fire when the
    # resulting slice is guaranteed to be empty.  Both indices must sit
    # on the same out-of-range side, OR the literal pair makes first >
    # last after clamping, so no element is touched.
    if cmd_name == "lrange":
        if len(args) != 3 or len(arg_tokens) < 3:
            return []
        first_pos, last_pos = 1, 2
    else:  # lreplace
        if len(args) < 3 or len(arg_tokens) < 3:
            return []
        first_pos, last_pos = 1, 2

    first_text = args[first_pos]
    last_text = args[last_pos]
    first_tok = arg_tokens[first_pos]
    last_tok = arg_tokens[last_pos]

    first_literal = not _has_subst(first_text, first_tok) and _is_literal_index(first_text)
    last_literal = not _has_subst(last_text, last_tok) and _is_literal_index(last_text)
    if not (first_literal and last_literal):
        return []

    first_val = _resolve_index(first_text, length)
    last_val = _resolve_index(last_text, length)
    if first_val is None or last_val is None:
        return []

    # Both indices below the list OR both above the list -> empty slice.
    both_below = first_val < 0 and last_val < 0
    both_above = first_val >= length and last_val >= length
    # Clamped first > clamped last -> empty slice.
    clamped_first = max(0, min(first_val, length - 1)) if length > 0 else 0
    clamped_last = max(0, min(last_val, length - 1)) if length > 0 else -1
    empty_after_clamp = length > 0 and clamped_first > clamped_last
    # Or the list is empty to begin with.
    list_empty = length == 0

    if not (both_below or both_above or empty_after_clamp or list_empty):
        return []

    if cmd_name == "lrange":
        verb = "lrange slice is empty"
    elif both_below:
        # Tcl clamps both indices to 0 and inserts the replacement at the
        # start — i.e. a prepend rather than the replacement the author
        # likely intended.
        verb = "lreplace prepends instead of replacing (both indices resolve before the list)"
    elif both_above:
        # Clamps to length — appends.
        verb = "lreplace appends instead of replacing (both indices resolve past the list)"
    else:
        verb = "lreplace touches no element (first > last after clamping)"
    msg = (
        f"{verb}: first='{first_text}' resolves to {first_val}, "
        f"last='{last_text}' resolves to {last_val} "
        f"(list has {length} element{'s' if length != 1 else ''})."
    )
    diagnostics.append(
        Diagnostic(
            range=Range(start=first_tok.start, end=last_tok.end),
            message=msg,
            severity=Severity.WARNING,
            code="W230",
        )
    )
    return diagnostics


@diag(
    code="W231",
    description="Constant list index out of range — lset raises a runtime error.",
    section="warning",
    ai_category="control_flow",
)
def check_lset_index_out_of_range(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W231: Warn on ``lset`` with constant index known to be out of range.

    Unlike ``lindex``, ``lset`` raises "index ... out of range" at runtime.
    Negative literal integer indices always error regardless of list length.
    """
    if cmd_name != "lset":
        return []
    # lset varName ?index ...? value -- at least varName, index, value.
    if len(args) < 3 or len(arg_tokens) < 3:
        return []

    # Multi-index forms (``lset L i j ... v``) address *nested* sublists;
    # index ``j`` is evaluated against ``lindex $L i``'s length, not
    # against ``$L``.  We only have the top-level length, so when more
    # than one index is present we fire only on *plain negative* literals
    # (those are invalid at every nesting level) and leave length-based
    # checks to the single-index form.
    index_positions = list(range(1, len(args) - 1))
    nested = len(index_positions) > 1

    # Try to resolve the list length via a recent literal `set var {...}`.
    var_name = args[0]
    list_len = (
        None
        if nested
        else _infer_list_length_from_recent_set(source, var_name, arg_tokens[0].start.offset)
    )

    diagnostics: list[Diagnostic] = []
    for pos in index_positions:
        idx_text = args[pos]
        idx_tok = arg_tokens[pos]
        if _has_subst(idx_text, idx_tok) or not _is_literal_index(idx_text):
            continue
        stripped = idx_text.strip()

        # Plain negative integer: always errors.
        if _INT_RE.match(stripped) and int(stripped) < 0:
            diagnostics.append(
                Diagnostic(
                    range=Range(start=idx_tok.start, end=idx_tok.end),
                    message=(
                        f"lset index '{stripped}' is negative; "
                        f"raises 'index out of range' at runtime."
                    ),
                    severity=Severity.WARNING,
                    code="W231",
                )
            )
            continue

        # ``lset`` accepts ``length`` (append position) as well as
        # ``[0, length)`` — e.g. ``end+1`` on a length-N list.  Only
        # indices *past* the append slot or below zero are runtime errors.
        if list_len is not None:
            resolved = _resolve_index(stripped, list_len)
            if resolved is None:
                continue
            if resolved < 0 or resolved > list_len:
                diagnostics.append(
                    Diagnostic(
                        range=Range(start=idx_tok.start, end=idx_tok.end),
                        message=(
                            f"lset index '{stripped}' "
                            f"{_describe_index(resolved, list_len)}; "
                            f"raises 'index out of range' at runtime."
                        ),
                        severity=Severity.WARNING,
                        code="W231",
                    )
                )
    return diagnostics


_LIST_SET_RE = re.compile(r"(?:^|\n)\s*set\s+(\w+)\s+(\{[^{}]*\})\s*(?=\n|$|;)")


def _infer_list_length_from_recent_set(
    source: str,
    var_name: str,
    before_offset: int,
) -> int | None:
    """Return the literal length of *var_name* if a recent ``set`` assigned it.

    Conservative, scope-aware heuristic:

    * Scans ``source[:before_offset]`` with a cheap regex for
      ``set <var> {literal}`` forms (O(n); full command segmentation
      would be quadratic across many ``lset`` calls).
    * Only accepts a match when the text between the match and the
      ``lset`` stays at brace depth zero — i.e. no unclosed ``{`` or
      ``}`` intervenes.  This rejects matches made in a *deeper* scope
      (a ``proc`` body, ``namespace eval`` block, or nested braces)
      that do not affect the variable the ``lset`` actually sees.
    * The regex already excludes values containing ``{`` / ``}``, so
      the captured literal cannot hide additional braces that would
      confuse the depth count.
    """
    if before_offset <= 0 or not var_name:
        return None
    best_length: int | None = None
    for m in _LIST_SET_RE.finditer(source, 0, before_offset):
        if m.group(1) != var_name:
            continue
        # Only trust the match if the surrounding scope stays flat
        # (no unclosed braces, no `proc`/`namespace eval` crossings).
        between = source[m.end() : before_offset]
        if not _scope_is_flat(between):
            continue
        elements = _split_list_literal(m.group(2)[1:-1])
        if elements is None:
            best_length = None
            continue
        best_length = len(elements)
    return best_length


_SCOPE_MARKERS_RE = re.compile(r"\b(?:proc|namespace\s+eval|apply|try)\b")


def _scope_is_flat(between: str) -> bool:
    """Return True if *between* stays at brace depth 0 and introduces no
    ``proc`` / ``namespace eval`` / ``apply`` / ``try`` scope.

    Because the captured ``set`` value excluded braces, any ``{`` we
    encounter here opens a new scope.  If any such scope remains open
    at the end of *between*, or any scope-introducing keyword appears,
    the trailing ``lset`` does not share the variable's originating
    scope and we must back off.
    """
    if _SCOPE_MARKERS_RE.search(between):
        return False
    depth = 0
    i = 0
    n = len(between)
    while i < n:
        ch = between[i]
        if ch == "\\" and i + 1 < n:
            i += 2
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth < 0:
                return False
        i += 1
    return depth == 0


# W232: string index out of range
# -----------------------------------------------------------------------------


@diag(
    code="W232",
    description=(
        "Constant string index out of range — string index/range/replace/insert "
        "silently return empty or no-op."
    ),
    section="warning",
    ai_category="control_flow",
)
def check_string_index_out_of_range(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W232: Warn when a constant ``string`` index is out of range.

    Covers ``string index``, ``string range``, ``string replace``, and
    ``string insert``.  Fires in two cases:

    * The string argument is a literal and a constant index resolves
      outside ``[0, length)``.
    * The index is a plain negative integer literal (always invalid,
      regardless of the string contents).
    """
    if cmd_name != "string" or len(args) < 2:
        return []
    sub = args[0]
    if sub not in ("index", "range", "replace", "insert"):
        return []

    # Argument layout:
    #   string index    STR idx
    #   string range    STR first last
    #   string replace  STR first last ?newstring?
    #   string insert   STR idx newstring
    if sub == "index":
        min_args = 3
    elif sub in ("range", "replace"):
        min_args = 4
    else:  # insert
        min_args = 4

    if len(args) < min_args or len(arg_tokens) < min_args:
        return []

    str_tok = arg_tokens[1]
    str_text = args[1]
    # Only use source length as the runtime length when it is safe to
    # do so: braced strings (``{...}``) perform no backslash processing,
    # and ESC words without any backslash are also byte-identical to
    # their runtime form.  An ESC with a backslash may have a source
    # length larger than its runtime length (``\n`` is 2 source chars,
    # 1 runtime char); we back off in that case rather than emit a
    # diagnostic against the wrong length.
    if _has_subst(str_text, str_tok) or not _is_braced_or_esc(str_tok):
        str_len = None
    elif str_tok.type is TokenType.STR:
        str_len = len(str_text)
    elif "\\" in str_text:
        str_len = None
    else:
        str_len = len(str_text)

    diagnostics: list[Diagnostic] = []

    if sub in ("index", "insert"):
        # Single-index subcommands.  ``string index`` returns empty on
        # any out-of-range hit; ``string insert`` is only flagged on
        # plain negative literals (Tcl silently clamps other overshoots
        # to a sensible position).
        pos = 2
        idx_text = args[pos]
        idx_tok = arg_tokens[pos]
        if _has_subst(idx_text, idx_tok) or not _is_literal_index(idx_text):
            return diagnostics
        stripped = idx_text.strip()

        if _INT_RE.match(stripped) and int(stripped) < 0:
            diagnostics.append(
                Diagnostic(
                    range=Range(start=idx_tok.start, end=idx_tok.end),
                    message=(
                        f"string {sub}: index '{stripped}' is negative; result is empty or a no-op."
                    ),
                    severity=Severity.WARNING,
                    code="W232",
                )
            )
            return diagnostics

        if sub == "index" and str_len is not None:
            resolved = _resolve_index(stripped, str_len)
            if resolved is None:
                return diagnostics
            if not (0 <= resolved < str_len):
                diagnostics.append(
                    Diagnostic(
                        range=Range(start=idx_tok.start, end=idx_tok.end),
                        message=(
                            f"string index: '{stripped}' "
                            f"{_describe_index_string(resolved, str_len)}; "
                            f"returns empty string."
                        ),
                        severity=Severity.WARNING,
                        code="W232",
                    )
                )
        return diagnostics

    # ``string range`` and ``string replace`` share a (first, last)
    # pair.  Flag only when the slice is provably empty (or the
    # replace is a no-op).
    first_pos, last_pos = 2, 3
    first_text = args[first_pos]
    last_text = args[last_pos]
    first_tok = arg_tokens[first_pos]
    last_tok = arg_tokens[last_pos]

    first_literal = not _has_subst(first_text, first_tok) and _is_literal_index(first_text)
    last_literal = not _has_subst(last_text, last_tok) and _is_literal_index(last_text)
    if not (first_literal and last_literal):
        return diagnostics

    # Plain negative literals on both indices -> always empty / no-op,
    # even when the string is dynamic.
    first_int = _INT_RE.match(first_text.strip())
    last_int = _INT_RE.match(last_text.strip())
    if first_int and last_int and int(first_text.strip()) < 0 and int(last_text.strip()) < 0:
        diagnostics.append(
            Diagnostic(
                range=Range(start=first_tok.start, end=last_tok.end),
                message=(
                    f"string {sub}: both indices are negative "
                    f"('{first_text}', '{last_text}'); "
                    f"{'slice is empty' if sub == 'range' else 'replace is a no-op'}."
                ),
                severity=Severity.WARNING,
                code="W232",
            )
        )
        return diagnostics

    if str_len is None:
        return diagnostics

    first_val = _resolve_index(first_text, str_len)
    last_val = _resolve_index(last_text, str_len)
    if first_val is None or last_val is None:
        return diagnostics

    both_below = first_val < 0 and last_val < 0
    both_above = first_val >= str_len and last_val >= str_len
    clamped_first = max(0, min(first_val, str_len - 1)) if str_len > 0 else 0
    clamped_last = max(0, min(last_val, str_len - 1)) if str_len > 0 else -1
    empty_after_clamp = str_len > 0 and clamped_first > clamped_last
    str_empty = str_len == 0

    if both_below or both_above or empty_after_clamp or str_empty:
        verb = "slice is empty" if sub == "range" else "replace is a no-op"
        diagnostics.append(
            Diagnostic(
                range=Range(start=first_tok.start, end=last_tok.end),
                message=(
                    f"string {sub}: {verb}: "
                    f"first='{first_text}' resolves to {first_val}, "
                    f"last='{last_text}' resolves to {last_val} "
                    f"(string has {str_len} character{'s' if str_len != 1 else ''})."
                ),
                severity=Severity.WARNING,
                code="W232",
            )
        )
    return diagnostics


def _describe_index_string(resolved: int, length: int) -> str:
    if resolved < 0:
        return f"resolves to {resolved} (before start of string)"
    return f"resolves to {resolved} (string has {length} character{'s' if length != 1 else ''})"


# Loop termination analysis (W240 / W241 / W242)
# -----------------------------------------------------------------------------


def _strip_braces(s: str) -> str:
    t = s.strip()
    if t.startswith("{") and t.endswith("}"):
        return t[1:-1].strip()
    return t


_TRUE_LITERALS = frozenset(("1", "true", "yes", "on", "!0"))
_FALSE_LITERALS = frozenset(("0", "false", "no", "off", "!1"))


def _condition_constant(cond: str) -> bool | None:
    """Return True/False if *cond* is a constant-true/-false expression."""
    c = _strip_braces(cond).lower()
    if c in _TRUE_LITERALS:
        return True
    if c in _FALSE_LITERALS:
        return False
    # Simple numeric: non-zero -> True, zero -> False.
    try:
        v = float(c)
        return v != 0.0
    except ValueError:
        return None


_BREAK_RE = re.compile(r"(?<![\w:-])(break|return|error|exit)\b")
_INCR_RE = re.compile(r"\bincr\s+(\w+)(?:\s+(-?\d+))?")
_SET_ASSIGN_RE = re.compile(r"\bset\s+(\w+)\b")


def _body_may_exit(body_text: str) -> bool:
    """Return True if *body* contains a break/return/error/exit (shallow scan)."""
    return _BREAK_RE.search(body_text) is not None


@diag(
    code="W240",
    description="Loop condition is a constant false — body never executes.",
    section="warning",
    ai_category="control_flow",
)
@diag(
    code="W241",
    description=(
        "Loop is provably infinite — constant-true condition with no "
        "break/return, zero/wrong-direction counter step."
    ),
    section="warning",
    ai_category="control_flow",
)
@diag(
    code="W242",
    description=(
        "Loop termination cannot be proven — counter not provably modified "
        "by the loop body or step."
    ),
    section="hint",
    ai_category="control_flow",
    default=False,
)
def check_loop_termination(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
) -> list[Diagnostic]:
    """W240 / W241 / W242: analyse ``while`` and ``for`` loops.

    * **W240** -- the condition is a constant-false literal, so the body is
      never entered.
    * **W241** -- the condition is a constant-true literal and the body
      contains no ``break`` / ``return`` / ``error`` / ``exit``, or the
      ``for`` step cannot make the condition false (step = 0, step in the
      wrong direction, ``incr`` by 0).
    * **W242** (default off) -- termination is *not* provably infinite but
      the analyser cannot see any write to the loop variable used in the
      condition; flagged as a HINT so authors can add a comment or rework.

    The analysis is intentionally shallow: it inspects the literal text of
    the condition and body.  If the condition is dynamic or the counter
    name cannot be recovered, no diagnostic is emitted (avoids false
    positives).
    """
    if cmd_name not in ("while", "for"):
        return []

    if cmd_name == "while":
        if len(args) < 2 or len(arg_tokens) < 2:
            return []
        cond_text = args[0]
        body_text = args[1]
        cond_tok = arg_tokens[0]
        init_text = ""
        step_text = ""
    else:  # for
        if len(args) < 4 or len(arg_tokens) < 4:
            return []
        init_text = args[0]
        cond_text = args[1]
        step_text = args[2]
        body_text = args[3]
        cond_tok = arg_tokens[1]

    diagnostics: list[Diagnostic] = []
    const = _condition_constant(cond_text)

    if const is False:
        diagnostics.append(
            Diagnostic(
                range=Range(start=cond_tok.start, end=cond_tok.end),
                message=(f"{cmd_name} condition is constant false; body never executes."),
                severity=Severity.WARNING,
                code="W240",
            )
        )
        return diagnostics

    if const is True:
        if not _body_may_exit(body_text):
            diagnostics.append(
                Diagnostic(
                    range=Range(start=cond_tok.start, end=cond_tok.end),
                    message=(
                        f"{cmd_name} is provably infinite: condition is constant "
                        f"true and body has no break/return/error/exit."
                    ),
                    severity=Severity.WARNING,
                    code="W241",
                )
            )
        return diagnostics

    # Heuristic analysis for `for {init} {cond} {step} body`.
    if cmd_name == "for":
        infinite = _for_is_provably_infinite(init_text, cond_text, step_text, body_text)
        if infinite is not None:
            diagnostics.append(
                Diagnostic(
                    range=Range(start=cond_tok.start, end=cond_tok.end),
                    message=f"for loop is provably infinite: {infinite}",
                    severity=Severity.WARNING,
                    code="W241",
                )
            )
            return diagnostics

    # W242: counter variable in condition but no step / body write visible.
    # Report the diagnostic on the *condition* token so the range is
    # consistent with W240/W241 (and IRULE5003); the condition is what
    # cannot be proven to change, not the body.
    var = _extract_counter_name(cond_text)
    if var is not None:
        modifies = _loop_modifies_var(var, step_text, body_text)
        if not modifies:
            diagnostics.append(
                Diagnostic(
                    range=Range(start=cond_tok.start, end=cond_tok.end),
                    message=(
                        f"{cmd_name} termination cannot be proven: variable "
                        f"'{var}' in the condition is never modified by the "
                        f"step or body."
                    ),
                    severity=Severity.HINT,
                    code="W242",
                )
            )
    return diagnostics


_SIMPLE_COUNTER_RE = re.compile(
    r"""
    \s*
    \$\{?(?P<v>\w+)\}?
    \s* (?P<op><=|>=|<|>|==|!=|eq|ne) \s*
    (?P<bound>-?\d+)
    \s*
    """,
    re.VERBOSE,
)
_SIMPLE_COUNTER_REV_RE = re.compile(
    r"""
    \s*
    (?P<bound>-?\d+)
    \s* (?P<op><=|>=|<|>|==|!=|eq|ne) \s*
    \$\{?(?P<v>\w+)\}?
    \s*
    """,
    re.VERBOSE,
)

# Compound-condition markers that mean the infinity proof is unsound:
# ``&&`` / ``||`` combine predicates, ``?:`` introduces a branch, and
# ``!`` can invert the sense of the comparison.
_COMPOUND_MARKERS_RE = re.compile(r"&&|\|\||\?|(?<![<>=!])!(?!=)")


def _parse_simple_for_cond(cond: str) -> tuple[str, str, int] | None:
    """Return (var, op, bound) if *cond* is exactly ``$v OP literal``
    or ``literal OP $v``.

    The regex is anchored (``fullmatch``) and compound operators
    (``&&``, ``||``, ``?:``, ``!``) cause an early return — so a
    compound condition such as ``$i < 10 && 0`` is never reduced to
    a simple counter form and thus does not trigger the W241 infinity
    proof.
    """
    c = _strip_braces(cond)
    if _COMPOUND_MARKERS_RE.search(c):
        return None
    m = _SIMPLE_COUNTER_RE.fullmatch(c)
    if m is not None:
        return m.group("v"), m.group("op"), int(m.group("bound"))
    m = _SIMPLE_COUNTER_REV_RE.fullmatch(c)
    if m is not None:
        # Flip operator so the variable is on the left.
        flip = {"<": ">", ">": "<", "<=": ">=", ">=": "<=", "==": "==", "!=": "!="}
        op = flip.get(m.group("op"), m.group("op"))
        return m.group("v"), op, int(m.group("bound"))
    return None


def _parse_init_var_value(init: str) -> tuple[str, int] | None:
    """Return (var, value) from an init clause like ``set i 0``."""
    text = _strip_braces(init).strip()
    m = re.match(r"^\s*set\s+(\w+)\s+(-?\d+)\s*$", text)
    if m is None:
        return None
    return m.group(1), int(m.group(2))


def _parse_step_incr(step: str) -> tuple[str, int] | None:
    """Return (var, delta) from a step clause like ``incr i`` or ``incr i -2``."""
    text = _strip_braces(step).strip()
    m = re.match(r"^\s*incr\s+(\w+)(?:\s+(-?\d+))?\s*$", text)
    if m is None:
        return None
    delta = int(m.group(2)) if m.group(2) is not None else 1
    return m.group(1), delta


def _for_is_provably_infinite(
    init: str,
    cond: str,
    step: str,
    body: str,
) -> str | None:
    """If we can prove the ``for`` loop never terminates, return a reason.

    Only fires on a narrow, high-precision shape:
    ``for {set v INT} {$v OP INT} {incr v INT} body`` with no write to ``v``
    elsewhere in the body.
    """
    parsed_cond = _parse_simple_for_cond(cond)
    parsed_init = _parse_init_var_value(init)
    parsed_step = _parse_step_incr(step)
    if parsed_cond is None or parsed_init is None or parsed_step is None:
        return None
    var_c, op, bound = parsed_cond
    var_i, start = parsed_init
    var_s, delta = parsed_step
    if var_c != var_i or var_c != var_s:
        return None

    # If the body writes to the counter, we cannot reason about termination.
    if _body_writes_var(body, var_c) or _body_may_exit(body):
        return None

    # Step of zero with condition initially true -> infinite.
    if delta == 0 and _cond_true_at(op, start, bound):
        return "step is zero ('incr' with 0) and condition holds on entry"

    # Wrong-direction step: moving away from the bound.
    if op in ("<", "<="):
        if _cond_true_at(op, start, bound) and delta < 0:
            return (
                f"counter ${var_c} starts at {start}, moves by {delta} "
                f"per step, and compares {op} {bound} (never reached)"
            )
    if op in (">", ">="):
        if _cond_true_at(op, start, bound) and delta > 0:
            return (
                f"counter ${var_c} starts at {start}, moves by {delta} "
                f"per step, and compares {op} {bound} (never reached)"
            )
    if op in ("!=", "ne"):
        # $v != N with a step that skips N -> never equal -> infinite.
        if delta == 0 and start != bound:
            return f"counter ${var_c} never changes and !={bound}"
        if delta != 0 and start != bound:
            diff = bound - start
            if diff * delta < 0:
                # Step goes away from bound.
                return (
                    f"counter ${var_c} starts at {start}, moves by {delta} "
                    f"per step, never reaches {bound}"
                )
            if diff % delta != 0:
                return (
                    f"counter ${var_c} starts at {start}, moves by {delta} "
                    f"per step, never exactly equals {bound}"
                )
    return None


def _cond_true_at(op: str, value: int, bound: int) -> bool:
    """Evaluate a simple comparison at a concrete value."""
    if op == "<":
        return value < bound
    if op == "<=":
        return value <= bound
    if op == ">":
        return value > bound
    if op == ">=":
        return value >= bound
    if op in ("==", "eq"):
        return value == bound
    if op in ("!=", "ne"):
        return value != bound
    return False


def _body_writes_var(body: str, var: str) -> bool:
    """Shallow scan: does *body* contain ``set var`` or ``incr var`` or ``lset var``?"""
    if re.search(rf"\bset\s+{re.escape(var)}\b", body):
        return True
    if re.search(rf"\bincr\s+{re.escape(var)}\b", body):
        return True
    if re.search(rf"\blset\s+{re.escape(var)}\b", body):
        return True
    if re.search(rf"\bappend\s+{re.escape(var)}\b", body):
        return True
    if re.search(rf"\blappend\s+{re.escape(var)}\b", body):
        return True
    return False


def _extract_counter_name(cond: str) -> str | None:
    """Return the first ``$var`` referenced by a condition expression."""
    c = _strip_braces(cond)
    m = re.search(r"\$\{?(\w+)\}?", c)
    if m is None:
        return None
    return m.group(1)


def _loop_modifies_var(var: str, step: str, body: str) -> bool:
    """Return True if the step or body updates *var* in a visible way."""
    if step:
        parsed = _parse_step_incr(step)
        if parsed is not None and parsed[0] == var:
            return True
        if _body_writes_var(_strip_braces(step), var):
            return True
    return _body_writes_var(body, var)
