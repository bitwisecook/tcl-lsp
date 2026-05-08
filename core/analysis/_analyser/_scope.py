from __future__ import annotations

import logging
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from ...common.naming import (
    is_brace_substitutable,
)
from ...common.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from ...common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ...common.naming import (
    split_array_name as _split_array_name,
)
from ...common.ranges import position_from_relative, range_from_token
from ...parsing.substitution import backslash_subst as _backslash_subst
from ...parsing.tokens import Token, TokenType
from ..semantic_model import (
    Diagnostic,
    Range,
    RegexPattern,
    Scope,
    Severity,
    VarDef,
)

log = logging.getLogger(__name__)


class _AnalyserScopeMixin(_Base):
    """Variable & scope tracking helpers."""

    def _set_const_string(
        self, var_name: str, value: str, value_range: Range, scope: Scope
    ) -> None:
        """Record a constant string assignment for *var_name* in *scope*."""
        sid = id(scope)
        if sid not in self._const_strings:
            self._const_strings[sid] = {}
        self._const_strings[sid][var_name] = (value, value_range)

    def _clear_const_string(self, var_name: str, scope: Scope) -> None:
        """Remove constant value knowledge for *var_name* (re-assigned dynamically)."""
        sid = id(scope)
        scope_dict = self._const_strings.get(sid)
        if scope_dict is not None:
            scope_dict.pop(var_name, None)

    def _lookup_const_string(self, var_name: str, scope: Scope) -> str | None:
        """Look up a constant string value for *var_name*, searching up the scope chain."""
        s: Scope | None = scope
        while s is not None:
            scope_dict = self._const_strings.get(id(s))
            if scope_dict is not None and var_name in scope_dict:
                return scope_dict[var_name][0]
            s = s.parent
        return None

    def _record_defining_set_as_regex(self, var_name: str, scope: Scope, command: str) -> None:
        """Mark the definition site of *var_name* as a regex pattern.

        Searches up the scope chain for the constant-string entry and, if
        found, adds a :class:`RegexPattern` pointing at the *value* range
        (not the variable name) so the semantic token provider can highlight
        the literal value as a regex.
        """
        s: Scope | None = scope
        while s is not None:
            scope_dict = self._const_strings.get(id(s))
            if scope_dict is not None and var_name in scope_dict:
                const_val, val_range = scope_dict[var_name]
                self.result.regex_patterns.append(
                    RegexPattern(
                        range=val_range,
                        pattern=const_val,
                        command=command,
                    )
                )
                return
            s = s.parent

    def _define_vars_from_list(
        self,
        var_list_text: str,
        tok: Token,
        scope: Scope,
    ) -> None:
        """Define variables from a varList token, giving each its own range."""
        text = tok.text
        span = tok.end.offset - tok.start.offset + 1
        content_offset = 1 if span > len(text) else 0
        base_line = tok.start.line
        base_col = tok.start.character + content_offset
        base_off = tok.start.offset + content_offset

        search_start = 0
        for var_name in var_list_text.split():
            if not var_name:
                continue
            idx = text.find(var_name, search_start)
            if idx >= 0:
                start_pos = position_from_relative(
                    text,
                    idx,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_off,
                )
                end_pos = position_from_relative(
                    text,
                    idx + len(var_name) - 1,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_off,
                )
                self._define_var(
                    var_name,
                    tok,
                    scope,
                    warn_if_unused=True,
                    definition_range=Range(start=start_pos, end=end_pos),
                )
                search_start = idx + len(var_name)
            else:
                self._define_var(var_name, tok, scope, warn_if_unused=True)

    def _namespace_from_scope(self, scope: Scope) -> str:
        """Derive the current namespace string from a scope chain.

        Results are cached on the analyser instance by scope identity so
        that repeated calls for the same scope (common during alias
        resolution) avoid re-walking the parent chain.
        """
        sid = id(scope)
        cached = self._ns_cache.get(sid)
        if cached is not None:
            return cached
        parts: list[str] = []
        s: Scope | None = scope
        while s is not None:
            if s.kind == "namespace":
                parts.append(s.name)
            s = s.parent
        if not parts:
            self._ns_cache[sid] = "::"
            return "::"
        parts.reverse()
        # Build the namespace path — each part may itself be relative
        # or absolute, so join them the same way the lowerer does.
        ns = "::"
        for p in parts:
            if p.startswith("::"):
                ns = _normalise_qualified_name(p)
            else:
                ns = _normalise_qualified_name(f"{ns}::{p}")
        self._ns_cache[sid] = ns
        return ns

    def _record_var_read(self, name: str, read_range: Range, scope: Scope) -> None:
        """Record a variable read for go-to-definition / find-references."""
        base_name = _normalise_var_name(name)
        if not base_name:
            return

        _base, element = _split_array_name(name)

        if base_name in scope.variables:
            scope.variables[base_name].references.append(read_range)
            if element is not None:
                scope.variables[base_name].array_indices.add(element)
            return

        # Cross-rule variables (::var and static::var) live in global scope.
        if (
            base_name.startswith("::") or base_name.startswith("static::")
        ) and base_name in self.result.global_scope.variables:
            self.result.global_scope.variables[base_name].references.append(read_range)
            if element is not None:
                self.result.global_scope.variables[base_name].array_indices.add(element)
            return
        # W210 is now emitted by the SSA-based analysis in
        # _emit_cfg_ssa_diagnostics_for_function.

    def _walk_scopes(self, scope: Scope):
        yield scope
        for child in scope.children:
            yield from self._walk_scopes(child)

    def _define_var(
        self,
        name: str,
        tok: Token,
        scope: Scope,
        *,
        warn_if_unused: bool = False,
        definition_range: Range | None = None,
    ) -> None:
        """Record a variable definition in the current scope."""
        base_name = _normalise_var_name(name)
        if not base_name:
            return

        _base, element = _split_array_name(name)

        first_definition = base_name not in scope.variables
        if first_definition:
            scope.variables[base_name] = VarDef(
                name=base_name,
                definition_range=definition_range or range_from_token(tok),
                warn_if_unused=warn_if_unused,
            )
            self.result.all_variables[f"{scope.name}::{base_name}"] = scope.variables[base_name]
            if element is not None:
                scope.variables[base_name].array_indices.add(element)
        else:
            if warn_if_unused:
                scope.variables[base_name].warn_if_unused = True
            if element is not None:
                scope.variables[base_name].array_indices.add(element)

        # Warn (once, at first definition) when the name is unreachable
        # via Tcl's ``$``-substitution forms.  The variable is creatable
        # via the command-word's backslash-subst (``set "weird}name" 1``,
        # ``set "back\\" 1``) and readable via commands that take a
        # variable *name* (``set name`` / ``[set "name"]`` / ``info
        # exists`` / ``upvar``), but neither ``$name`` nor ``${name}``
        # can fetch the value.
        #
        # Reachability is computed via :func:`is_brace_substitutable`
        # which mirrors Tcl 9.0.3's ``Tcl_ParseVarName`` brace-form
        # parser exactly (see ``docs/kcs/kcs-tcl-corner-cases.md``).
        if first_definition:
            site_range = definition_range or range_from_token(tok)
            # Compute the *actual* runtime var name by applying Tcl
            # backslash substitution.  Our scope.variables key is the
            # raw segmenter-arg text (which keeps escapes verbatim), but
            # Tcl 9.0.3 creates the var under the subst-applied name --
            # so e.g. ``set "back\\\\slash" 1`` produces var ``back\slash``
            # at runtime even though our scope key is ``back\\\\slash``.
            # W215's reachability check must use the runtime name.
            #
            # IMPORTANT: backslash substitution is suppressed inside
            # *braced* command words.  Tcl preserves every byte verbatim
            # in ``{...}`` -- so ``set {a\}b} 1`` creates the runtime
            # var ``a\}b`` (4 chars including the literal backslash) and
            # ``set "a\}b" 1`` creates ``a}b`` (3 chars, ``\}`` -> ``}``).
            # ``Token.type == STR`` flags a braced literal; ``in_quote``
            # flags an arg lifted from inside double quotes.  Bare and
            # quoted words both go through backslash subst at command
            # parsing; braced words don't.
            apply_backslash_subst = "\\" in base_name and tok.type is not TokenType.STR
            runtime_name = _backslash_subst(base_name) if apply_backslash_subst else base_name
            if element is not None and "\\" in element and tok.type is not TokenType.STR:
                runtime_element: str | None = _backslash_subst(element)
            else:
                runtime_element = element
            if not is_brace_substitutable(runtime_name):
                # Identify the worst offending feature for the message.
                if "}" in runtime_name:
                    detail = (
                        "the brace form ``${name}`` ends at the first "
                        "``}`` not preceded by ``\\`` (and the bare form "
                        "stops at the first non-word character)"
                    )
                elif runtime_name.endswith("\\"):
                    detail = (
                        "the brace form ``${name}`` would read the "
                        "trailing ``\\`` as the start of a 2-char escape "
                        "and run out of input -- missing close-brace"
                    )
                elif "{" in runtime_name:
                    detail = (
                        "the brace form ``${name}`` has unbalanced ``{`` "
                        "and runs out of input -- missing close-brace"
                    )
                else:
                    detail = "the brace form ``${name}`` cannot match this name"
                self.result.diagnostics.append(
                    Diagnostic(
                        range=site_range,
                        message=(
                            f"variable name ``{runtime_name}`` is not reachable via "
                            "$-substitution; it can still be created/read via "
                            '``set name`` / ``[set "name"]`` / ``info exists`` / '
                            f"``upvar``, but {detail}"
                        ),
                        severity=Severity.WARNING,
                        code="W215",
                    )
                )
            if runtime_element is not None and ")" in runtime_element:
                self.result.diagnostics.append(
                    Diagnostic(
                        range=site_range,
                        message=(
                            "array element index contains ')'; the element can be created "
                            'and read via ``set arr(idx) ...`` / ``[set "arr(idx)"]``, but '
                            "is not reachable via $-substitution (``$arr(idx)`` reads up "
                            "to the first ``)`` and stops there)"
                        ),
                        severity=Severity.WARNING,
                        code="W215",
                    )
                )
