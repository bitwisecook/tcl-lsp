"""Parser-backed iRules object reference extraction.

The actual mapping of iRule commands to BIG-IP object kinds lives in
:mod:`dialects.f5.bigip.irules_object_refs` as a declarative table.  This
module is a walker: it segments the iRule body into commands, asks the
table for each command's object-reference arg positions, and resolves
those positions to literal object names — including names that flow
through ``set <var> <literal>`` bindings.
"""

from __future__ import annotations

from dataclasses import dataclass

from analyser.semantic_model import Range
from compiler.parsing.command_segmenter import SegmentedCommand, segment_commands
from compiler.parsing.lexer import TclLexer
from compiler.parsing.token_positions import token_content_base
from compiler.registry.runtime import ArgRole, arg_indices_for_role
from shared.ranges import position_from_relative
from shared.tokens import Token, TokenType

from .irules_object_refs import resolve_object_ref_args


@dataclass(frozen=True, slots=True)
class IrulesObjectReference:
    """One iRules object reference resolved from a literal command argument."""

    name: str
    kinds: tuple[str, ...]
    command: str
    argument_index: int  # 0-based index after command name
    range: Range


# Variable-name → (literal_value, range) bindings carried through linear
# copy-propagation within an event-handler scope.  Branching control
# flow widens the binding to ``None`` (overdefined) so we don't emit
# false positives.  Each nested body inherits a snapshot of its parent
# scope, then mutations stay local to the child.
@dataclass(slots=True)
class _BindingScope:
    bindings: dict[str, tuple[str, Range] | None]

    @classmethod
    def empty(cls) -> "_BindingScope":
        return cls(bindings={})

    def child(self) -> "_BindingScope":
        return _BindingScope(bindings=dict(self.bindings))

    def set_const(self, var: str, value: str, rng: Range) -> None:
        self.bindings[var] = (value, rng)

    def widen(self, var: str) -> None:
        # Mark the binding as overdefined so downstream lookups fail-closed.
        self.bindings[var] = None

    def lookup(self, var: str) -> tuple[str, Range] | None:
        return self.bindings.get(var)


def _normalise_literal_name(text: str) -> str | None:
    """Normalise a literal word into a potential BIG-IP object name."""
    clean = text.strip()
    if not clean:
        return None
    if clean.startswith("$") or clean.startswith("["):
        return None
    if any(ch.isspace() for ch in clean):
        return None
    return clean


def _literal_arg_value(cmd: SegmentedCommand, arg_index: int) -> tuple[str, Range] | None:
    """Return normalised literal argument text and source range for one arg."""
    word_index = arg_index + 1
    if word_index >= len(cmd.argv):
        return None
    if word_index >= len(cmd.single_token_word) or not cmd.single_token_word[word_index]:
        return None
    tok = cmd.argv[word_index]
    if tok.type not in (TokenType.ESC, TokenType.STR):
        return None

    raw = tok.text
    left = 0
    right = len(raw) - 1
    while left <= right and raw[left].isspace():
        left += 1
    while right >= left and raw[right].isspace():
        right -= 1
    if left > right:
        return None

    base_offset, base_line, base_col = token_content_base(tok)
    start = position_from_relative(
        raw,
        left,
        base_line=base_line,
        base_col=base_col,
        base_offset=base_offset,
    )
    end = position_from_relative(
        raw,
        right,
        base_line=base_line,
        base_col=base_col,
        base_offset=base_offset,
    )
    name = _normalise_literal_name(raw[left : right + 1])
    if name is None:
        return None
    return (name, Range(start=start, end=end))


def _var_token_name(tok: Token) -> str | None:
    """Return the variable name when *tok* is a single ``$var`` substitution.

    The lexer strips the leading ``$`` from a ``VAR`` token, so the
    token text already holds the variable name (or ``${name}`` form).
    Returns ``None`` for array-element references (``foo(bar)``) — we
    don't currently track array slots in the binding scope.
    """
    if tok.type is not TokenType.VAR:
        return None
    text = tok.text.strip()
    if not text:
        return None
    name = text
    if name.startswith("{") and name.endswith("}"):
        name = name[1:-1]
    if not name:
        return None
    if "(" in name:
        return None
    return name


def _resolve_arg_value(
    cmd: SegmentedCommand,
    arg_index: int,
    scope: _BindingScope,
) -> tuple[str, Range] | None:
    """Resolve a command argument to ``(literal_name, range)`` if possible.

    Tries the literal value first; if the arg is a pure ``$var``
    substitution, falls back to the binding scope's most recent
    constant assignment for that variable.  Returns ``None`` for
    everything else (command substitutions, mixed literal-plus-var,
    overdefined bindings, ...).
    """
    literal = _literal_arg_value(cmd, arg_index)
    if literal is not None:
        return literal

    word_index = arg_index + 1
    if word_index >= len(cmd.argv):
        return None
    if word_index >= len(cmd.single_token_word) or not cmd.single_token_word[word_index]:
        return None
    tok = cmd.argv[word_index]
    var_name = _var_token_name(tok)
    if var_name is None:
        return None
    binding = scope.lookup(var_name)
    if binding is None:
        return None
    value, _decl_range = binding
    # Anchor the resolved reference to the use site (the $var token), not
    # the declaration site, so editors highlight where the cleanup script
    # would land.
    base_offset, base_line, base_col = token_content_base(tok)
    raw = tok.text
    use_range = Range(
        start=position_from_relative(
            raw,
            0,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        ),
        end=position_from_relative(
            raw,
            len(raw) - 1,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        ),
    )
    return (value, use_range)


def _record_set_binding(cmd: SegmentedCommand, scope: _BindingScope) -> None:
    """If *cmd* is ``set <var> <literal>`` (single-token), record the binding."""
    if cmd.name != "set" or len(cmd.args) < 2:
        return
    # `set var` (one arg) is a read, not a write.
    var = cmd.args[0]
    if not var or any(ch.isspace() for ch in var) or var.startswith("$"):
        return
    if "(" in var:
        # Array element — not tracked.
        return
    literal = _literal_arg_value(cmd, 1)
    if literal is None:
        # Non-literal RHS: widen any existing binding so later lookups
        # don't pick up a stale value.
        scope.widen(var)
        return
    value, rng = literal
    scope.set_const(var, value, rng)


def _append_reference(
    out: list[IrulesObjectReference],
    cmd: SegmentedCommand,
    scope: _BindingScope,
    *,
    arg_index: int,
    kinds: tuple[str, ...],
) -> None:
    resolved = _resolve_arg_value(cmd, arg_index, scope)
    if resolved is None:
        return
    name, rng = resolved
    out.append(
        IrulesObjectReference(
            name=name,
            kinds=kinds,
            command=cmd.name,
            argument_index=arg_index,
            range=rng,
        )
    )


def _walk_irules_commands(
    source: str,
    out: list[IrulesObjectReference],
    *,
    body_token: Token | None = None,
    rule_module: str | None = None,
    scope: _BindingScope | None = None,
) -> None:
    if scope is None:
        scope = _BindingScope.empty()

    commands = segment_commands(source, body_token=body_token)
    for cmd in commands:
        # Resolve declared references before mutating the binding table —
        # the references must see the bindings that were live *at* this
        # call, not after any same-line ``set`` re-bind it might do.
        for arg_index, kinds in resolve_object_ref_args(
            cmd.name, cmd.args, rule_module=rule_module
        ):
            _append_reference(out, cmd, scope, arg_index=arg_index, kinds=kinds)

        # Update the binding table *after* the references are recorded so
        # ``set p /Common/foo; pool $p`` and ``pool $p; set p /Common/foo``
        # are handled as Tcl semantics dictate (linearly).
        _record_set_binding(cmd, scope)

        body_indices = arg_indices_for_role(cmd.name, cmd.args, ArgRole.BODY)
        expr_indices = arg_indices_for_role(cmd.name, cmd.args, ArgRole.EXPR)
        recursed_cmd_tokens: set[tuple[int, int]] = set()
        for body_idx in body_indices:
            word_index = body_idx + 1
            if word_index >= len(cmd.argv):
                continue
            tok = cmd.argv[word_index]
            if tok.type not in (TokenType.STR, TokenType.CMD):
                continue
            if not tok.text.strip():
                continue
            # Each body opens a child scope that inherits parent bindings.
            # Mutations inside the body do not leak back to the parent.
            _walk_irules_commands(
                tok.text,
                out,
                body_token=tok,
                rule_module=rule_module,
                scope=scope.child(),
            )
            if tok.type is TokenType.CMD:
                recursed_cmd_tokens.add((tok.start.offset, tok.end.offset))

        for expr_idx in expr_indices:
            word_index = expr_idx + 1
            if word_index >= len(cmd.argv):
                continue
            tok = cmd.argv[word_index]
            if tok.type not in (TokenType.STR, TokenType.ESC):
                continue
            base_offset, base_line, base_col = token_content_base(tok)
            lexer = TclLexer(
                tok.text,
                base_offset=base_offset,
                base_line=base_line,
                base_col=base_col,
            )
            while True:
                inner = lexer.get_token()
                if inner is None:
                    break
                if inner.type is not TokenType.CMD or not inner.text.strip():
                    continue
                key = (inner.start.offset, inner.end.offset)
                if key in recursed_cmd_tokens:
                    continue
                recursed_cmd_tokens.add(key)
                _walk_irules_commands(
                    inner.text,
                    out,
                    body_token=inner,
                    rule_module=rule_module,
                    scope=scope.child(),
                )

        for tok in cmd.all_tokens:
            if tok.type is not TokenType.CMD:
                continue
            key = (tok.start.offset, tok.end.offset)
            if key in recursed_cmd_tokens:
                continue
            if not tok.text.strip():
                continue
            _walk_irules_commands(
                tok.text,
                out,
                body_token=tok,
                rule_module=rule_module,
                scope=scope.child(),
            )


def extract_irules_object_references(
    source: str,
    *,
    body_token: Token | None = None,
    rule_module: str | None = None,
) -> list[IrulesObjectReference]:
    """Extract BIG-IP object references from iRules source using Tcl parsing.

    Resolves both literal arguments (``pool /Common/foo``) and
    constant variables propagated through ``set`` (``set p /Common/foo;
    pool $p``).  Variable resolution operates per scope: each event
    handler / nested body owns a binding table that inherits from its
    parent and is widened to "overdefined" when a re-assignment from a
    non-literal source (e.g. ``set p [HTTP::host]``) makes the binding
    unsafe to trust.
    """
    refs: list[IrulesObjectReference] = []
    _walk_irules_commands(source, refs, body_token=body_token, rule_module=rule_module)
    refs.sort(
        key=lambda ref: (
            ref.range.start.line,
            ref.range.start.character,
            ref.range.end.line,
            ref.range.end.character,
            ref.name,
        )
    )
    return refs
