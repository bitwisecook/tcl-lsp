"""Shared variable-reference scanning utilities for compiler passes."""

from __future__ import annotations

from dataclasses import dataclass

from ..commands.registry.runtime import ArgRole, arg_indices_for_role
from ..common.naming import normalise_var_name
from ..parsing.lexer import TclLexer
from ..parsing.tokens import TokenType


@dataclass(frozen=True, slots=True)
class VarScanOptions:
    include_var_read_roles: bool = False
    recurse_cmd_substitutions: bool = True


class VarReferenceScanner:
    """Scan Tcl words/scripts for referenced variable names."""

    def __init__(self, options: VarScanOptions | None = None) -> None:
        self._options = options or VarScanOptions()

    def scan_word(self, text: str) -> set[str]:
        """Scan one Tcl word for variable references."""
        return self.scan_script(text)

    def scan_script(self, source: str) -> set[str]:
        """Scan a Tcl script for variable references."""
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

        if self._options.include_var_read_roles:
            vars_found |= self._scan_var_read_role_names(source)

        return vars_found

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
