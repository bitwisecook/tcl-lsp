"""Inline variable — replace a single-use variable with its value."""

from __future__ import annotations

from analyser import analyse
from shared.position import offset_at_position
from shared.tokens import TokenType

from . import RefactoringEdit, RefactoringResult
from ._spans import command_span_offsets, find_command_at, offsets_to_position, token_end_offset


def inline_variable(
    source: str,
    line: int,
    character: int,
) -> RefactoringResult | None:
    """Inline the variable defined at the cursor position.

    Only inlines when:
    - The cursor is on a ``set var value`` command.
    - The variable has exactly one reference after the definition.

    Returns *None* when inlining is not safe.
    """
    result = analyse(source)

    # Find the ``set`` command at the cursor (recursing into nested bodies).
    cmd = find_command_at(source, line, character, predicate="set")
    if cmd is None:
        return None
    if len(cmd.texts) < 3:
        return None  # ``set var`` (read form) — nothing to inline

    var_name = cmd.texts[1]
    # The value word, taken verbatim from source.  ``token_end_offset``
    # widens the span to include the closing quote/brace/bracket.
    value_tok = cmd.argv[2]
    value_text = source[value_tok.start.offset : token_end_offset(source, value_tok)]

    # Find the variable definition in the semantic model.
    var_def = None
    for scope in _walk_scopes(result.global_scope):
        if var_name in scope.variables:
            vd = scope.variables[var_name]
            if vd.definition_range.start.line == cmd.range.start.line:
                var_def = vd
                break
    if var_def is None:
        return None

    # Only inline if used exactly once.
    if len(var_def.references) != 1:
        return None
    ref = var_def.references[0]

    # Delete the ``set`` command (token span, plus a trailing newline).
    cmd_start, cmd_end = command_span_offsets(source, cmd)
    if cmd_end < len(source) and source[cmd_end] == "\n":
        cmd_end += 1
    set_sl, set_sc, set_el, set_ec = offsets_to_position(source, cmd_start, cmd_end)
    delete_edit = RefactoringEdit(
        start_line=set_sl,
        start_character=set_sc,
        end_line=set_el,
        end_character=set_ec,
        new_text="",
    )

    # Locate the reference's variable token and its enclosing word — this is
    # what tells us, from the tokens alone, whether the reference is a
    # standalone word or interpolated inside a larger word.
    ctx = _reference_token(source, ref)
    if ctx is None:
        return None
    var_tok, word_is_single, word_start_offset = ctx

    # The ``$var`` / ``${var}`` span to replace.  The VAR token already
    # starts at ``$``; its end omits the closing ``}`` of the braced form,
    # so re-add it when present.
    ref_start = var_tok.start.offset
    ref_end = token_end_offset(source, var_tok)
    if (
        source[ref_start : ref_start + 2] == "${"
        and ref_end < len(source)
        and source[ref_end] == "}"
    ):
        ref_end += 1
    ref_sl, ref_sc, ref_el, ref_ec = offsets_to_position(source, ref_start, ref_end)

    # A standalone word keeps the value verbatim (preserving its quotes/braces
    # so the word stays a single word).  When the reference is interpolated
    # inside a larger word, splice the unquoted value content instead —
    # keeping the delimiters would yield ``"hello "world""``.  A bare
    # (unquoted) concatenation can only take a value with no whitespace,
    # otherwise the result would split into multiple words.
    if word_is_single:
        new_text = value_text
    else:
        inner = _dequote(value_text)
        in_quoted_string = source[word_start_offset : word_start_offset + 1] == '"'
        if not in_quoted_string and any(ch.isspace() for ch in inner):
            return None
        new_text = inner

    replace_edit = RefactoringEdit(
        start_line=ref_sl,
        start_character=ref_sc,
        end_line=ref_el,
        end_character=ref_ec,
        new_text=new_text,
    )

    return RefactoringResult(
        title=f"Inline variable '${var_name}'",
        edits=(delete_edit, replace_edit),
        kind="refactor.inline",
    )


def _reference_token(source: str, ref):
    """Return ``(var_token, word_is_single, word_start_offset)`` for *ref*.

    Resolves the reference through the segmenter so the decision is made from
    tokens, not text scanning: ``word_is_single`` is true when the reference
    is its own word (``$x``) and false when it is interpolated inside a larger
    word (``"a $x b"`` / ``foo$x``).
    """
    cmd = find_command_at(source, ref.start.line, ref.start.character)
    if cmd is None:
        return None
    ref_off = offset_at_position(source, ref.start.line, ref.start.character)

    var_tok = None
    for tok in cmd.all_tokens:
        if tok.type is TokenType.VAR and tok.start.offset <= ref_off <= token_end_offset(
            source, tok
        ):
            var_tok = tok
            break
    if var_tok is None:
        return None

    for word_tok, is_single in zip(cmd.argv, cmd.single_token_word):
        word_start = word_tok.start.offset
        word_end = token_end_offset(source, word_tok)
        if word_start <= var_tok.start.offset < word_end:
            return var_tok, is_single, word_start
    return var_tok, True, var_tok.start.offset


def _dequote(value_text: str) -> str:
    """Strip one layer of matching ``"..."`` or ``{...}`` delimiters."""
    if len(value_text) >= 2 and value_text[0] == value_text[-1] == '"':
        return value_text[1:-1]
    if len(value_text) >= 2 and value_text[0] == "{" and value_text[-1] == "}":
        return value_text[1:-1]
    return value_text


def _walk_scopes(scope):
    """Yield *scope* and all descendant scopes."""
    yield scope
    for child in scope.children:
        yield from _walk_scopes(child)
