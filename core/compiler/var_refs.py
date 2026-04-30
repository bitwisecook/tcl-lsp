"""Shared variable-reference scanning utilities for compiler passes."""

from __future__ import annotations

from collections import OrderedDict
from dataclasses import dataclass

from ..commands.registry.runtime import ArgRole, arg_indices_for_role
from ..common.naming import normalise_var_name
from ..parsing.lexer import TclLexer
from ..parsing.tokens import TokenType

_DEFAULT_CACHE_SIZE = 512


def _strip_outer_braces(text: str) -> str:
    """Strip one layer of outer ``{…}`` braces from a Tcl word.

    Used when recursing into BODY/EXPR args that are typically passed as a
    single brace-quoted word — the lexer sees the braces as a STR token and
    won't tokenise the contents otherwise.
    """
    s = text.strip()
    if len(s) >= 2 and s.startswith("{") and s.endswith("}"):
        return s[1:-1]
    return s


@dataclass(frozen=True, slots=True)
class VarScanOptions:
    """Options controlling :class:`VarReferenceScanner` behaviour.

    Attributes:
        include_var_read_roles: When True, also report variable names that
            appear at ``ArgRole.VAR_READ`` positions (e.g. ``info exists
            varName``) in addition to ``$var`` substitutions.
        recurse_cmd_substitutions: When True, recurse into the source of
            ``[cmd-substitution]`` tokens.
        recurse_body_args: When True, walk each command and recurse into
            ``ArgRole.BODY`` / ``ArgRole.EXPR`` args, scanning their
            brace-stripped contents as Tcl scripts.  Used by SSA when
            scanning opaque ``IRBarrier`` / ``IRCall`` bodies that are not
            lowered into the CFG (e.g. ``catch``, ``dict for``, deferred
            ``foreach``) — without this, ``$var`` references inside the
            braced body are hidden from the lexer.
    """

    include_var_read_roles: bool = False
    recurse_cmd_substitutions: bool = True
    recurse_body_args: bool = False


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
        self._cache: OrderedDict[str, frozenset[str]] = OrderedDict()
        self._cache_size = cache_size

    def scan_word(self, text: str) -> frozenset[str]:
        """Scan one Tcl word for variable references."""
        return self.scan_script(text)

    def scan_script(self, source: str) -> frozenset[str]:
        """Scan a Tcl script for variable references (LRU-cached)."""
        cached = self._cache.get(source)
        if cached is not None:
            self._cache.move_to_end(source)
            return cached
        result = self._scan_script_uncached(source)
        self._cache[source] = result
        if len(self._cache) > self._cache_size:
            self._cache.popitem(last=False)
        return result

    def clear_cache(self) -> None:
        """Drop all cached results."""
        self._cache.clear()

    def _scan_script_uncached(self, source: str) -> frozenset[str]:
        """Scan without cache — called on cache miss."""
        vars_found: set[str] = set()
        lexer = TclLexer(source)

        while True:
            tok = lexer.get_token()
            if tok is None:
                break
            if tok.type is TokenType.VAR:
                name = normalise_var_name(tok.text)
                if name:
                    vars_found.add(name)
            elif tok.type is TokenType.CMD and self._options.recurse_cmd_substitutions and tok.text:
                vars_found |= self.scan_script(tok.text)

        if self._options.include_var_read_roles or self._options.recurse_body_args:
            vars_found |= self._scan_var_read_role_names(source)

        return frozenset(vars_found)

    def _scan_var_read_role_names(self, source: str) -> set[str]:
        result: set[str] = set()
        lexer = TclLexer(source)
        words: list[str] = []
        prev_type = TokenType.EOL

        def flush_command() -> None:
            if not words:
                return
            cmd_name = words[0]
            args = words[1:]
            for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.VAR_READ)):
                if idx < len(args):
                    name = normalise_var_name(args[idx])
                    if name:
                        result.add(name)
            if self._options.recurse_body_args:
                # Recurse into BODY/EXPR args so vars referenced inside
                # nested braced control-flow (``if {$x>0} {…}`` etc.) are
                # surfaced.  Pure-write roles (``set x 5``, ``lassign``,
                # ``regexp -- … x``) are deliberately NOT counted as uses
                # — W214's contract is "parameter never read", and writing
                # to a parameter without reading it is exactly that.
                for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.BODY)):
                    if idx < len(args):
                        result.update(self.scan_script(_strip_outer_braces(args[idx])))
                for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.EXPR)):
                    if idx < len(args):
                        result.update(self.scan_script(_strip_outer_braces(args[idx])))

        for tok in lexer.tokenise_all():
            if tok.type in (TokenType.EOL, TokenType.EOF):
                flush_command()
                words = []
                prev_type = tok.type
                continue
            if tok.type is TokenType.SEP:
                prev_type = tok.type
                continue
            if prev_type in (TokenType.SEP, TokenType.EOL):
                words.append(tok.text)
            else:
                if words:
                    words[-1] += tok.text
                else:
                    words.append(tok.text)
            prev_type = tok.type
        flush_command()
        return result
