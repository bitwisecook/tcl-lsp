"""Shared variable-reference scanning utilities for compiler passes."""

from __future__ import annotations

from collections import OrderedDict
from collections.abc import Sequence
from dataclasses import dataclass

from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.green_tree import tokenise
from compiler.parsing.token_scanning import scan_command_substitutions
from compiler.registry.runtime import (
    REGISTRY,
    ArgRole,
    arg_indices_for_role,
    signature_overlay_epoch,
)
from shared.naming import normalise_var_name, split_array_name
from shared.tokens import Token, TokenType

_DEFAULT_CACHE_SIZE = 512


@dataclass(frozen=True, slots=True)
class VarScanOptions:
    """Options controlling :class:`VarReferenceScanner` behaviour.

    Attributes:
        include_var_read_roles: When True, also report variable names that
            appear at ``ArgRole.VAR_READ`` positions (e.g. ``info exists
            varName``) in addition to ``$var`` substitutions.
        recurse_cmd_substitutions: When True, recurse into the source of
            ``[cmd-substitution]`` tokens.
        recurse_into_script_roles: When True, recursively scan BODY-role
            and EXPR-role argument words (as scripts) for nested variable
            references.  Driven by the command registry, so plain braced
            data words (e.g. ``set msg {$lit}``) are left alone — only
            argument positions known to be scripts/expressions are
            descended into.  Used by SSA when scanning opaque ``IRBarrier``
            bodies that are not lowered into the CFG (e.g. ``dict for``)
            so ``$var`` references inside nested braced control-flow are
            still observed.
        recurse_into_expr_roles: Like ``recurse_into_script_roles`` but
            descends ``ArgRole.EXPR`` positions **only**, not BODY scripts.
            An expr argument's ``$var`` references are read in the current
            scope when the command evaluates (e.g.
            ``incr i [expr {$digit * $w}]`` reads ``digit`` and ``w``),
            whereas BODY scripts are deferred/nested contexts whose reads
            must not be attributed to the enclosing statement.  Use this for
            statement use-tracking; ``recurse_into_script_roles`` implies it.
    """

    include_var_read_roles: bool = False
    recurse_cmd_substitutions: bool = True
    recurse_into_script_roles: bool = False
    recurse_into_expr_roles: bool = False
    include_reads_before_write: bool = False
    """When True, a read-modify-write command (``incr``/``append``/``lappend``)
    reports its ``ArgRole.VAR_WRITE`` *target* as a read too — it reads the prior
    value before updating it.  Used to recover the target read of such a command
    buried in a command substitution (``lappend out [incr i $j]`` reads ``i``),
    which is otherwise invisible and makes the feeding ``set i 0`` look dead."""


class VarReferenceScanner:
    """Scan Tcl words/scripts for referenced variable names.

    Results are cached in a bounded LRU cache keyed by source text.
    The same word/script strings are scanned repeatedly across SSA,
    GVN, and interprocedural passes, so caching avoids redundant
    lexer creation and tokenisation.
    """

    def __init__(
        self,
        options: VarScanOptions | None = None,
        cache_size: int = _DEFAULT_CACHE_SIZE,
    ) -> None:
        self._options = options or VarScanOptions()
        self._cache: OrderedDict[tuple, frozenset[str]] = OrderedDict()
        self._cache_size = cache_size

    def scan_word(self, text: str) -> frozenset[str]:
        """Scan one Tcl word for variable references."""
        return self.scan_script(text)

    def scan_script(self, source: str) -> frozenset[str]:
        """Scan a Tcl script for variable references (LRU-cached).

        The result depends on the active signature/stub/dialect overlay
        (``arg_indices_for_role`` consults it), so the cache key folds in
        ``signature_overlay_epoch()``: the same text scanned before and inside a
        ``stub_signature_scope`` must not reuse the pre-stub result.
        """
        key = (signature_overlay_epoch(), source)
        cached = self._cache.get(key)
        if cached is not None:
            self._cache.move_to_end(key)
            return cached
        result = self._scan_script_uncached(source)
        self._cache[key] = result
        if len(self._cache) > self._cache_size:
            self._cache.popitem(last=False)
        return result

    def clear_cache(self) -> None:
        """Drop all cached results."""
        self._cache.clear()

    def _scan_script_uncached(self, source: str) -> frozenset[str]:
        """Scan without cache — called on cache miss."""
        vars_found: set[str] = set()
        # Tokenise once and share the token stream across the variable,
        # var-read-role, and script-role scans below — each of those
        # previously built its own ``TclLexer`` over the same source.
        # Route through the shared per-analysis memo so the independent
        # scanner singletons (SSA, GVN, interprocedural) reuse one
        # tokenisation of the same body text rather than re-lexing it each.
        # Variable names are position-independent, so lexing at base 0 is
        # fine even though the memo keys on base offset.
        tokens, _ = tokenise(source, 0, 0, 0)

        for tok in tokens:
            if tok.type is TokenType.VAR:
                name = normalise_var_name(tok.text)
                if name:
                    vars_found.add(name)
                # An array element ``$arr($idx)`` substitutes its *index* at
                # access time, so a ``$``/``[…]`` in the parentheses is a real
                # read (``$state($whom)`` reads ``whom``).  ``normalise_var_name``
                # keeps only the array name, so recover the index refs here.
                text = tok.text
                lparen = text.find("(")
                if lparen != -1 and text.endswith(")"):
                    index = text[lparen + 1 : -1]
                    if "$" in index or "[" in index:
                        vars_found |= self.scan_word(index)
            elif tok.type is TokenType.CMD and self._options.recurse_cmd_substitutions and tok.text:
                vars_found |= self.scan_script(tok.text)

        if self._options.include_var_read_roles:
            vars_found |= self._scan_var_read_role_names(source)

        if self._options.recurse_into_script_roles or self._options.recurse_into_expr_roles:
            vars_found |= self._scan_script_role_args(tokens)

        return frozenset(vars_found)

    def _scan_script_role_args(self, tokens: Sequence[Token]) -> set[str]:
        """Walk a pre-tokenised script command-by-command, recursing into
        BODY/EXPR args.

        Only argument positions registered as ``ArgRole.BODY`` (Tcl scripts)
        or ``ArgRole.EXPR`` (expressions) are descended into.  Plain braced
        data words are left alone, so this does not introduce false reads
        for literals like ``set msg {$unused}``.
        """
        result: set[str] = set()
        words: list[str] = []
        prev_type = TokenType.EOL

        def flush_command() -> None:
            if not words:
                return
            cmd_name = words[0]
            args = words[1:]
            recurse_indices = arg_indices_for_role(cmd_name, args, ArgRole.EXPR)
            if self._options.recurse_into_script_roles:
                recurse_indices = recurse_indices | arg_indices_for_role(
                    cmd_name, args, ArgRole.BODY
                )
            for virtual_idx in sorted(recurse_indices):
                if virtual_idx >= len(args):
                    continue
                inner = args[virtual_idx]
                # Strip a single layer of outer braces so the inner text is
                # tokenised as a script (TclLexer would otherwise emit STR
                # for the whole word and skip its contents).
                if len(inner) >= 2 and inner[0] == "{" and inner[-1] == "}":
                    inner = inner[1:-1]
                if inner:
                    result.update(self.scan_script(inner))

        for tok in tokens:
            if tok.type in (TokenType.EOL, TokenType.EOF):
                flush_command()
                words = []
                prev_type = tok.type
                continue
            if tok.type is TokenType.SEP:
                prev_type = tok.type
                continue
            # Reconstruct the original word text including braces so that
            # ``arg_indices_for_role`` and the brace-strip below see the
            # word exactly as it appeared in source.
            text = tok.text
            if tok.type is TokenType.STR:
                text = "{" + tok.text + "}"
            elif tok.type is TokenType.CMD:
                text = "[" + tok.text + "]"
            elif tok.type is TokenType.VAR:
                text = "$" + tok.text
            if prev_type in (TokenType.SEP, TokenType.EOL):
                words.append(text)
            else:
                if words:
                    words[-1] += text
                else:
                    words.append(text)
            prev_type = tok.type
        flush_command()
        return result

    def _scan_var_read_role_names(self, source: str) -> set[str]:
        result: set[str] = set()
        for seg in segment_commands(source):
            cmd_name = seg.name
            args = seg.args
            arg_tokens = seg.arg_tokens
            read_indices = set(arg_indices_for_role(cmd_name, args, ArgRole.VAR_READ))
            # A read-modify-write command (incr/append/lappend) reads its
            # VAR_WRITE target's prior value, so report it as a read too.
            if self._options.include_reads_before_write and REGISTRY.is_reads_variable_before_write(
                cmd_name
            ):
                read_indices |= set(arg_indices_for_role(cmd_name, args, ArgRole.VAR_WRITE))
            for idx in sorted(read_indices):
                if idx >= len(args) or idx >= len(arg_tokens):
                    continue
                # A substitution-named read target (``info exists [foo]`` /
                # ``array get $name``) computes the variable name at runtime —
                # it is not a literal read of ``foo`` / ``name``.  A literal
                # name with a substituted index (``info exists arr($i)``) still
                # reads array ``arr``, so only VAR/CMD-led words are dropped.
                if arg_tokens[idx].type in (TokenType.VAR, TokenType.CMD):
                    continue
                name = normalise_var_name(args[idx])
                if name:
                    result.add(name)
        return result


# A plain, fully-literal variable name (no substitutions, no specials): the
# only kind of write target we treat as a definite current-scope write when
# recovered from inside a command substitution.  ``catch {...} msg opts`` and
# ``regexp re $s m`` write literal names; ``scan $s %d $dyn`` writes a
# *runtime-named* variable (the value of ``$dyn``), so it must not be counted.
#
# A literal array element (``catch {...} msg(0)`` / ``regexp re $s a(0)``) is
# also a definite current-scope write — tclsh creates the array element, so a
# later ``$msg(0)`` read is not read-before-set.  ``normalise_var_name`` maps it
# to the array base (``msg``), which suppresses W210 for the array (suppress-only
# — never a false positive).  A *dynamic* element (``msg($i)``) still contains
# ``$`` and is rejected, as is any substitution-bearing name.
def _is_literal_name(word: str) -> bool:
    return bool(word) and all(c not in word for c in "$[{} \t")


def _collect_cmd_sub_writes(script: str, out: set[str]) -> None:
    """Collect literal ``ArgRole.VAR_WRITE`` argument names from every command
    in *script*, recursing into nested command substitutions."""
    tokens, _ = tokenise(script, 0, 0, 0)
    words: list[str] = []
    prev_type = TokenType.EOL

    def flush_command() -> None:
        if not words:
            return
        cmd_name = words[0]
        args = words[1:]
        for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.VAR_WRITE)):
            if idx < len(args) and _is_literal_name(args[idx]):
                name = normalise_var_name(args[idx])
                if name:
                    out.add(name)
        # An ``ArgRole.EXPR`` argument (``expr {…}``, ``if {cond}``, loop
        # conditions) evaluates command substitutions inside it at run time, so
        # a ``[catch … var]`` buried in an expr writes ``var`` in the current
        # scope too (e.g. ``set e [expr {[catch {…} tmp] || $tmp}]``).  The
        # expr text is one braced STR word, opaque to the word tokeniser, so
        # strip the outer braces and scan its command subs explicitly.
        expr_indices = set(arg_indices_for_role(cmd_name, args, ArgRole.EXPR))
        for idx, w in enumerate(args):
            if idx in expr_indices:
                inner = w[1:-1] if len(w) >= 2 and w[0] == "{" and w[-1] == "}" else w
                if "[" in inner:
                    _scan_word_cmd_subs(inner, out)
        # Recurse into any command substitutions appearing inside this
        # command's words (e.g. ``set e [catch {...} msg opts]``).
        for w in words:
            if "[" in w:
                _scan_word_cmd_subs(w, out)

    for tok in tokens:
        if tok.type in (TokenType.EOL, TokenType.EOF):
            flush_command()
            words = []
            prev_type = tok.type
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        text = tok.text
        if tok.type is TokenType.STR:
            text = "{" + tok.text + "}"
        elif tok.type is TokenType.CMD:
            text = "[" + tok.text + "]"
        elif tok.type is TokenType.VAR:
            text = "$" + tok.text
        if prev_type in (TokenType.SEP, TokenType.EOL):
            words.append(text)
        else:
            if words:
                words[-1] += text
            else:
                words.append(text)
        prev_type = tok.type
    flush_command()


def _scan_word_cmd_subs(word: str, out: set[str]) -> None:
    """Scan a single word for ``[...]`` command substitutions and collect the
    literal VAR_WRITE names written inside each."""
    for tok in scan_command_substitutions(word):
        if tok.text:
            _collect_cmd_sub_writes(tok.text, out)


def command_sub_write_names(text: str) -> frozenset[str]:
    """Literal variable names written by command substitutions inside *text*.

    A command substitution like ``[catch {...} msg opts]`` writes ``msg`` and
    ``opts`` in the *current* scope (verified vs tclsh).  The Tcl-level word
    scanner treats the whole ``[...]`` as one opaque value, so those writes are
    invisible to SSA def tracking and a later ``$msg`` read looks "read before
    set".  This recovers the written names (name-level only, literal targets
    only) so read-before-set can suppress them.  Recurses into nested subs.
    """
    if "[" not in text:
        return frozenset()
    out: set[str] = set()
    _scan_word_cmd_subs(text, out)
    return frozenset(out)


_EXISTENCE_TEST_CMDS = frozenset({"info", "::info", "array", "::array"})


def _collect_existence_tests(script: str, out: set[str]) -> None:
    """Collect variable names that are the target of an existence test
    (``info exists X`` / ``array exists X``) in *script*, recursing into nested
    command substitutions.  An array-element target (``info exists a(k)``)
    contributes its base array name (``a``)."""
    tokens, _ = tokenise(script, 0, 0, 0)
    words: list[str] = []
    prev_type = TokenType.EOL

    def flush_command() -> None:
        if not words:
            return
        if len(words) >= 3 and words[0] in _EXISTENCE_TEST_CMDS and words[1] == "exists":
            target = words[2]
            base, _ = split_array_name(target)
            if base and _is_literal_name(base):
                name = normalise_var_name(base)
                if name:
                    out.add(name)
        cmd_name = words[0]
        cmd_args = words[1:]
        # An ``info exists`` may hide inside an EXPR-role arg (``if {[info exists
        # x]}``, loop conditions) or a BODY-role script (``while {…} {… if {[info
        # exists x]} …}``) — both braced, so the word tokeniser treats the inner
        # ``[...]`` as literal.  Strip the braces and recurse so existence tests
        # at any nesting depth are recovered.
        script_indices = set(arg_indices_for_role(cmd_name, cmd_args, ArgRole.EXPR)) | set(
            arg_indices_for_role(cmd_name, cmd_args, ArgRole.BODY)
        )
        for idx, w in enumerate(cmd_args):
            if idx in script_indices:
                inner = w[1:-1] if len(w) >= 2 and w[0] == "{" and w[-1] == "}" else w
                _collect_existence_tests(inner, out)
            elif "[" in w:
                _scan_word_existence_tests(w, out)
        if "[" in cmd_name:
            _scan_word_existence_tests(cmd_name, out)

    for tok in tokens:
        if tok.type in (TokenType.EOL, TokenType.EOF):
            flush_command()
            words = []
            prev_type = tok.type
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        text = tok.text
        if tok.type is TokenType.STR:
            text = "{" + tok.text + "}"
        elif tok.type is TokenType.CMD:
            text = "[" + tok.text + "]"
        elif tok.type is TokenType.VAR:
            text = "$" + tok.text
        if prev_type in (TokenType.SEP, TokenType.EOL):
            words.append(text)
        else:
            if words:
                words[-1] += text
            else:
                words.append(text)
        prev_type = tok.type
    flush_command()


def _scan_word_existence_tests(word: str, out: set[str]) -> None:
    tokens, _ = tokenise(word, 0, 0, 0)
    for tok in tokens:
        if tok.type is TokenType.CMD and tok.text:
            _collect_existence_tests(tok.text, out)


def existence_test_names(text: str) -> frozenset[str]:
    """Variable names tested for existence (``info exists`` / ``array exists``)
    anywhere in *text*, including inside command substitutions.

    Referencing an *unset* variable in these predicates is legal and is the
    canonical test-before-use idiom (``if {![info exists x]} {set x …}``), so a
    read-before-set check must not treat such a name as an uninitialised read.
    Returned name-level (suppress-only), like :func:`command_sub_write_names`."""
    out: set[str] = set()
    _collect_existence_tests(text, out)
    return frozenset(out)


_FOREACH_CMDS = frozenset({"foreach", "::foreach", "lmap", "::lmap"})


def _add_write_name(target: str, out: set[str]) -> None:
    base, _ = split_array_name(target)
    if base and _is_literal_name(base):
        name = normalise_var_name(base)
        if name:
            out.add(name)


def _collect_body_writes(script: str, out: set[str]) -> None:
    """Collect variable names written by any command in a *body script* — direct
    ``ArgRole.VAR_WRITE`` targets (``set``/``gets``/``scan``/``catch`` result
    vars), ``foreach``/``lmap`` loop variables, and writes nested inside
    ``ArgRole.BODY`` scripts (``for`` init/next/body, ``if``/``while`` bodies)
    and command substitutions.

    Used to balance read recovery for *frozen* loops (``while``/``for`` with a
    command-substitution condition, kept as an opaque barrier): their body reads
    are recovered name-level, so their body writes must be too, or every
    body-local variable looks read-before-set."""
    tokens, _ = tokenise(script, 0, 0, 0)
    words: list[str] = []
    prev_type = TokenType.EOL

    def flush_command() -> None:
        if not words:
            return
        cmd_name = words[0]
        args = words[1:]
        for idx in arg_indices_for_role(cmd_name, args, ArgRole.VAR_WRITE):
            if 0 <= idx < len(args) and _is_literal_name(args[idx]):
                _add_write_name(args[idx], out)
        if cmd_name in _FOREACH_CMDS and len(args) >= 3:
            # foreach varList list ?varList list ...? body — loop vars are the
            # even-indexed args before the trailing body.
            for i in range(0, len(args) - 1, 2):
                vl = args[i]
                if len(vl) >= 2 and vl[0] == "{" and vl[-1] == "}":
                    vl = vl[1:-1]
                for v in vl.split():
                    _add_write_name(v, out)
        if cmd_name in ("try", "::try"):
            # try body ?on code varList body ...? ?trap pat varList body ...?
            # ?finally body? — each handler binds the result/options vars named
            # in its varList (these are not ArgRole.VAR_WRITE-tagged).
            i = 0
            while i < len(args):
                if args[i] in ("on", "trap") and i + 2 < len(args):
                    vl = args[i + 2]
                    if len(vl) >= 2 and vl[0] == "{" and vl[-1] == "}":
                        vl = vl[1:-1]
                    for v in vl.split():
                        _add_write_name(v, out)
                    i += 4
                else:
                    i += 1
        for idx in arg_indices_for_role(cmd_name, args, ArgRole.BODY):
            if 0 <= idx < len(args):
                w = args[idx]
                inner = w[1:-1] if len(w) >= 2 and w[0] == "{" and w[-1] == "}" else w
                _collect_body_writes(inner, out)
        for w in args:
            if "[" in w:
                _scan_word_cmd_subs(w, out)

    for tok in tokens:
        if tok.type in (TokenType.EOL, TokenType.EOF):
            flush_command()
            words = []
            prev_type = tok.type
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        text = tok.text
        if tok.type is TokenType.STR:
            text = "{" + tok.text + "}"
        elif tok.type is TokenType.CMD:
            text = "[" + tok.text + "]"
        elif tok.type is TokenType.VAR:
            text = "$" + tok.text
        if prev_type in (TokenType.SEP, TokenType.EOL):
            words.append(text)
        else:
            if words:
                words[-1] += text
            else:
                words.append(text)
        prev_type = tok.type
    flush_command()


def body_write_names(text: str) -> frozenset[str]:
    """Variable names written anywhere in the body script *text* (see
    :func:`_collect_body_writes`)."""
    out: set[str] = set()
    _collect_body_writes(text, out)
    return frozenset(out)


def scan_var_ref_forms(text: str) -> tuple[str, ...]:
    """Return the *full* variable-reference forms read in *text* — keeping the
    array-index suffix that :class:`VarReferenceScanner` normalises away.

    ``$a`` → ``"a"``, ``$a(k)`` → ``"a(k)"``, ``$state($whom)`` →
    ``"state($whom)"``.  Recurses into ``[...]`` command substitutions so reads
    nested in subs are included.  Unlike the name-set scanners, this preserves
    enough structure for :func:`compiler.var_resolve.resolve_place` to classify
    a scalar vs an array element vs a dynamic ref.
    """
    out: list[str] = []
    _collect_ref_forms(text, out)
    return tuple(out)


def _collect_ref_forms(text: str, out: list[str]) -> None:
    if not text:
        return
    tokens, _ = tokenise(text, 0, 0, 0)
    for tok in tokens:
        if tok.type is TokenType.VAR and tok.text:
            out.append(tok.text)
        elif tok.type is TokenType.CMD and tok.text:
            _collect_ref_forms(tok.text, out)
