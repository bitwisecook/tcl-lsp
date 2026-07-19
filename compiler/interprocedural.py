# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Interprocedural procedure summaries (Phase 5).

Conservative summaries are built per lowered proc to describe:
- side-effect / purity shape
- internal call graph edges
- constant return behaviour
- parameter sensitivity of return values
- safe static call folding opportunities
"""

# canonicalisation: audited #246

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from compiler.analysis_types import LatticeKind, LatticeValue
from compiler.cfg import CFGFunction, CFGReturn, build_cfg
from compiler.core_analyses import _expr_has_command as _expr_has_command_sub
from compiler.core_analyses import analyse_function
from compiler.eval_helpers import DECIMAL_INT_RE as _DECIMAL_INT_RE
from compiler.expr_ast import (
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
)
from compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRMethodDef,
    IRModule,
    IRProcedure,
    IRReturn,
    IRScript,
    IRSwitch,
    IRTry,
    IRWhile,
)
from compiler.lowering import lower_to_ir
from compiler.parsing.expr_parser import parse_expr
from compiler.parsing.green_tree import tokenise
from compiler.parsing.token_scanning import is_simple_scalar_var_word
from compiler.proc_arg_traits import (
    infer_param_traits,
    infer_param_traits_deep,
    merge_traits,
)
from compiler.registry.dialect import active_dialect
from compiler.registry.runtime import arg_indices_for_role, resolve_arg_role_map
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import EffectRegion, classify_side_effects
from compiler.ssa import SSAFunction, SSAValueKey, build_ssa, is_complexity_guarded
from compiler.static_loops import evaluate_expr_with_constants as _evaluate_expr
from compiler.var_refs import VarReferenceScanner, VarScanOptions
from shared.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from shared.naming import (
    normalise_var_name as _normalise_var_name,
)
from shared.naming import (
    split_array_name as _split_array_name,
)
from shared.proc_traits import ProcArgTrait
from shared.tokens import TokenType

if TYPE_CHECKING:
    from compiler.core_analyses import FunctionAnalysis

log = logging.getLogger(__name__)

_VAR_REF_SCANNER = VarReferenceScanner(
    VarScanOptions(
        include_var_read_roles=False,
        recurse_cmd_substitutions=True,
    )
)


@dataclass(frozen=True, slots=True)
class ProcSummary:
    qualified_name: str
    params: tuple[str, ...]
    arity: Arity
    calls: tuple[str, ...]
    has_barrier: bool
    has_unknown_calls: bool
    writes_global: bool
    pure: bool
    effect_reads: EffectRegion
    effect_writes: EffectRegion
    returns_constant: bool
    constant_return: int | float | bool | str | None
    return_depends_on_params: tuple[str, ...]
    return_passthrough_param: str | None
    can_fold_static_calls: bool
    param_traits: dict[str, frozenset[ProcArgTrait]] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class MethodSummary(ProcSummary):
    """Extended summary for OO methods with class context."""

    class_name: str = ""
    method_kind: str = "method"  # method/classmethod/constructor/destructor
    reads_instance_vars: frozenset[str] = frozenset()
    writes_instance_vars: frozenset[str] = frozenset()
    calls_my: tuple[str, ...] = ()  # methods called via `my method`
    calls_next: bool = False  # calls `next` (MRO chain dispatch)


@dataclass(frozen=True, slots=True)
class InterproceduralAnalysis:
    procedures: dict[str, ProcSummary]
    methods: dict[str, MethodSummary] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class ProcLocalSummary:
    """Procedure facts independent from transitive callee closure."""

    qualified_name: str
    params: tuple[str, ...]
    arity: Arity
    calls: tuple[str, ...]
    has_barrier: bool
    has_unknown_calls: bool
    writes_global: bool
    local_effect_reads: EffectRegion
    local_effect_writes: EffectRegion
    returns_constant: bool
    constant_return: int | float | bool | str | None
    return_depends_on_params: tuple[str, ...]
    return_passthrough_param: str | None


@dataclass(slots=True)
class _LocalFacts:
    calls: set[str]
    has_barrier: bool
    has_unknown_calls: bool
    writes_global: bool
    effect_reads: EffectRegion
    effect_writes: EffectRegion
    # Local variable names this proc has aliased into global / enclosing-
    # namespace scope via ``global`` / ``variable`` / ``upvar #0`` (or ``0``).
    # A write to any of these names mutates a variable the *caller* can see,
    # so it counts as ``writes_global`` even though the written name is bare.
    global_aliases: set[str] = field(default_factory=set)


def _global_alias_names(command: str, args: tuple[str, ...]) -> set[str] | None:
    """Local names a scope-aliasing *command* binds to global/namespace scope.

    Returns ``None`` when *command* is not a global-aliasing declaration.
    Returns the set of bound local names for ``global`` / ``variable`` /
    ``upvar #0`` (``0``); a sentinel-bearing set including ``""`` signals a
    dynamic/unbounded declaration the caller must treat pessimistically.

    ``upvar`` at any *other* level aliases a caller frame, not global scope,
    and is handled conservatively elsewhere — so it returns ``None`` here.
    """

    def _names(raw_names: list[str]) -> set[str]:
        out: set[str] = set()
        for raw in raw_names:
            head = raw.lstrip()[:1]
            if head in ("$", "["):
                # Dynamic alias target — can't bound the name set.
                out.add("")
                continue
            out.add(_normalise_var_name(raw))
        return out

    if command == "global":
        return _names(list(args))
    if command == "variable":
        # ``variable name ?value? name ?value? ...`` — names at even indices.
        return _names([args[i] for i in range(0, len(args), 2)])
    if command == "upvar":
        if not args:
            return None
        level = args[0].strip()
        # Only ``#0`` / ``0`` alias the *global* frame.  ``upvar`` with the
        # level omitted defaults to 1 (a caller frame), as do explicit
        # numeric levels — none of which is a global write.
        if level not in ("#0", "0"):
            return None
        # ``upvar #0 otherVar localVar ...`` — local names are every second
        # arg after the level.
        return _names([args[i] for i in range(2, len(args), 2)])
    return None


@dataclass(frozen=True, slots=True)
class _ReturnInfo:
    const_known: bool
    const_value: int | float | bool | str | None
    param_deps: frozenset[str]
    passthrough_param: str | None


def _namespace_parts_from_proc(qname: str) -> list[str]:
    parts = [p for p in _normalise_qualified_name(qname).split("::") if p]
    if len(parts) <= 1:
        return []
    return parts[:-1]


def resolve_internal_call(command: str, caller_qname: str, known: set[str]) -> str | None:
    if not command:
        return None

    if command.startswith("::"):
        qname = _normalise_qualified_name(command)
        return qname if qname in known else None

    if "::" in command:
        qname = _normalise_qualified_name(f"::{command}")
        return qname if qname in known else None

    ns_parts = _namespace_parts_from_proc(caller_qname)
    for depth in range(len(ns_parts), -1, -1):
        prefix = ns_parts[:depth]
        if prefix:
            candidate = "::" + "::".join(prefix + [command])
        else:
            candidate = f"::{command}"
        if candidate in known:
            return candidate
    return None


def resolve_call_target(
    command: str,
    args: tuple[str, ...] | list[str],
    caller_qname: str,
    known: set[str],
) -> str | None:
    """Resolve the target proc, seeing through iRules ``call`` indirection.

    For ``call myproc ...``, the real target is *args[0]*.
    For ``myproc ...`` (direct invocation), the target is *command*.

    Accepts both bare (``call``) and canonical (``::call``) command
    forms — callers may pass either ``IRCall.command`` (raw) or
    ``IRCall.canonical_command`` (qualified).  See issue #246.
    """
    if command in ("call", "::call") and args:
        return resolve_internal_call(args[0], caller_qname, known)
    return resolve_internal_call(command, caller_qname, known)


def _vars_in_script(source: str) -> frozenset[str]:
    return _VAR_REF_SCANNER.scan_script(source)


def _vars_in_word(text: str) -> frozenset[str]:
    return _VAR_REF_SCANNER.scan_word(text)


def _parse_literal_word(text: str) -> int | bool | str | None:
    stripped = text.strip()
    if _DECIMAL_INT_RE.fullmatch(stripped):
        try:
            return int(stripped)
        except ValueError:
            return None
    if stripped.lower() == "true":
        return True
    if stripped.lower() == "false":
        return False
    if "$" in stripped or "[" in stripped:
        return None
    return stripped


def _single_simple_var_word(text: str) -> str | None:
    stripped = text.strip()
    if not is_simple_scalar_var_word(stripped):
        return None
    return _normalise_var_name(stripped)


def _contains_command_substitution(text: str | None) -> bool:
    return bool(text and "[" in text)


def _apply_effect(facts: _LocalFacts, effect) -> None:
    reads, writes = effect.to_effect_regions()
    facts.effect_reads |= reads
    facts.effect_writes |= writes
    if effect.dynamic_barrier:
        facts.has_barrier = True
    if not effect.pure and effect.writes_any:
        facts.has_unknown_calls = True
    if bool(writes & EffectRegion.GLOBAL_STATE):
        facts.writes_global = True


def _strip_braces(text: str) -> str:
    stripped = text.strip()
    if len(stripped) >= 2 and stripped[0] == "{" and stripped[-1] == "}":
        return stripped[1:-1]
    return stripped


def _scan_script_text(
    text: str,
    *,
    caller_qname: str,
    known_procs: set[str],
    facts: _LocalFacts,
) -> None:
    """Lex *text* as a Tcl script and record call targets / side effects.

    Builds Tcl words from adjacent token fragments (``foo[bar]baz`` is
    one word, not three), skips comments, and recurses into both
    ``ArgRole.BODY`` arguments (catch / eval / if / while / for / …)
    and ``ArgRole.EXPR`` arguments (the expression itself is re-scanned
    so command subs inside conditions surface as call edges even when
    the BODY recursion reached this script through a nested path).
    """
    if not text:
        return
    lex_tokens, _ = tokenise(text, 0, 0, 0)
    # Words accumulate token fragments until a SEP/EOL boundary.
    current_words: list[_WordFragment] = []
    word_in_progress: _WordFragment | None = None

    def _flush_word() -> None:
        nonlocal word_in_progress
        if word_in_progress is not None:
            current_words.append(word_in_progress)
            word_in_progress = None

    for tok in lex_tokens:
        if tok.type is TokenType.EOL:
            _flush_word()
            if current_words:
                _process_command_words(
                    current_words,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
                current_words = []
            continue
        if tok.type is TokenType.SEP:
            _flush_word()
            continue
        if tok.type is TokenType.COMMENT:
            # Tcl comments are not commands.  The lexer emits these as
            # standalone tokens between EOLs, never adjacent to a word
            # fragment, so we simply drop them.
            continue
        if tok.type is TokenType.EOF:
            break
        if tok.type is TokenType.CMD:
            # Nested ``[...]`` substitution — scan its contents as a
            # script for side effects, but the substitution is part of
            # whatever word it's embedded in (``foo[bar]baz`` is one
            # word, not three).
            _scan_script_text(
                tok.text,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            if word_in_progress is not None:
                word_in_progress.is_static = False
            else:
                word_in_progress = _WordFragment(text="", is_braced=False, is_static=False)
            continue
        if tok.type is TokenType.VAR:
            # Variable substitution joins the surrounding word but
            # contributes no statically known text.
            if word_in_progress is not None:
                word_in_progress.is_static = False
            else:
                word_in_progress = _WordFragment(text="", is_braced=False, is_static=False)
            continue
        if tok.type is TokenType.EXPAND:
            # ``{*}`` argument expansion — we don't model expansion
            # statically, so just drop the marker.
            continue
        # ESC or STR — concrete textual fragment.
        if word_in_progress is None:
            word_in_progress = _WordFragment(
                text=tok.text,
                is_braced=tok.type is TokenType.STR,
                is_static=True,
            )
        else:
            word_in_progress.text += tok.text
            # If a word mixes braced and unbraced fragments, treat the
            # combined form as unbraced — _strip_braces is only safe on
            # a pure ``{...}`` literal.
            if tok.type is not TokenType.STR:
                word_in_progress.is_braced = False
    _flush_word()
    if current_words:
        _process_command_words(
            current_words,
            caller_qname=caller_qname,
            known_procs=known_procs,
            facts=facts,
        )


@dataclass(slots=True)
class _WordFragment:
    text: str
    is_braced: bool
    is_static: bool  # False if any VAR/CMD/EXPAND fragment merged in


def _process_command_words(
    words: list[_WordFragment],
    *,
    caller_qname: str,
    known_procs: set[str],
    facts: _LocalFacts,
) -> None:
    if not words:
        return
    head = words[0]
    if not head.is_static:
        # Dynamic command word (``$cmd ...``, ``[lookup] ...``) — we
        # cannot resolve the target statically, so apply a conservative
        # effect.  The inner CMD substitution has already been scanned
        # by ``_scan_script_text``.
        facts.has_unknown_calls = True
        return
    cmd_word = _strip_braces(head.text) if head.is_braced else head.text
    if not cmd_word:
        return
    arg_words = [w.text for w in words[1:]]
    cmd_args = tuple(arg_words)
    target = resolve_call_target(cmd_word, cmd_args, caller_qname, known_procs)
    if target is not None:
        facts.calls.add(target)
    else:
        _apply_effect(facts, classify_side_effects(cmd_word, cmd_args))

    role_map = resolve_arg_role_map(cmd_word, list(arg_words))
    for idx, roles in role_map.items():
        if not (0 <= idx < len(arg_words)):
            continue
        if ArgRole.BODY in roles:
            frag = words[idx + 1]
            if not frag.is_static:
                # ``$var`` / ``[cmd]`` body — the value isn't statically
                # known, so we cannot recurse into it.
                continue
            body_text = _strip_braces(frag.text) if frag.is_braced else frag.text
            _scan_script_text(
                body_text,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
        if ArgRole.EXPR in roles:
            # Expression argument — its command substitutions are call sites,
            # and a math function call ``Foo(...)`` dispatches to the command
            # ``::tcl::mathfunc::Foo`` (#958).  Parse the expression so both
            # surface as call-graph edges; ``_scan_expr_for_calls`` handles
            # ``[cmd]`` substitutions and ExprRaw fall-through identically to
            # the embedded-command scanner.
            frag = words[idx + 1]
            if not frag.is_static:
                continue
            expr_text = _strip_braces(frag.text) if frag.is_braced else frag.text
            try:
                expr_ast = parse_expr(expr_text, dialect=active_dialect())
            except Exception:
                expr_ast = None
            if expr_ast is not None:
                _scan_expr_for_calls(
                    expr_ast,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            else:
                _scan_embedded_commands(
                    expr_text,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )


def _scan_embedded_commands(
    text: str,
    *,
    caller_qname: str,
    known_procs: set[str],
    facts: _LocalFacts,
) -> None:
    lex_tokens, _ = tokenise(text, 0, 0, 0)
    for tok2 in lex_tokens:
        if tok2.type is not TokenType.CMD:
            continue
        _scan_script_text(
            tok2.text,
            caller_qname=caller_qname,
            known_procs=known_procs,
            facts=facts,
        )


def _scan_expr_for_calls(
    node: ExprNode,
    *,
    caller_qname: str,
    known_procs: set[str],
    facts: _LocalFacts,
) -> None:
    """Walk *node* and scan every ``[cmd ...]`` substitution as a script."""
    match node:
        case ExprCommand(text=text):
            # text includes surrounding ``[...]``.  Drop the brackets and
            # scan the inner script for call targets.
            inner = text.strip()
            if len(inner) >= 2 and inner[0] == "[" and inner[-1] == "]":
                inner = inner[1:-1]
            _scan_script_text(
                inner,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
        case ExprRaw(text=text):
            if "[" in text:
                _scan_embedded_commands(
                    text,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
        case ExprString(text=text):
            # Double-quoted operands undergo command substitution at
            # expr-evaluation time: ``if {"[q]" ne ""} {...}`` calls
            # ``q`` even though it appears inside quotes.  Scan for
            # embedded ``[...]`` substitutions.
            if "[" in text:
                _scan_embedded_commands(
                    text,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
        case ExprBinary(left=left, right=right):
            _scan_expr_for_calls(
                left, caller_qname=caller_qname, known_procs=known_procs, facts=facts
            )
            _scan_expr_for_calls(
                right, caller_qname=caller_qname, known_procs=known_procs, facts=facts
            )
        case ExprUnary(operand=operand):
            _scan_expr_for_calls(
                operand, caller_qname=caller_qname, known_procs=known_procs, facts=facts
            )
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            _scan_expr_for_calls(
                cond, caller_qname=caller_qname, known_procs=known_procs, facts=facts
            )
            _scan_expr_for_calls(
                tb, caller_qname=caller_qname, known_procs=known_procs, facts=facts
            )
            _scan_expr_for_calls(
                fb, caller_qname=caller_qname, known_procs=known_procs, facts=facts
            )
        case ExprCall(function=func, args=args):
            # A ``expr`` function call ``Foo(...)`` dispatches to the command
            # ``::tcl::mathfunc::Foo``.  When such a proc is defined, record the
            # call-graph edge so a proc calling a user-defined math function is
            # not treated as a leaf (#958).  Built-in functions (``sin``, …)
            # have no proc and are simply skipped by the membership test.
            math_target = f"::tcl::mathfunc::{func}"
            if math_target in known_procs:
                facts.calls.add(math_target)
            for arg in args:
                _scan_expr_for_calls(
                    arg, caller_qname=caller_qname, known_procs=known_procs, facts=facts
                )


def _scan_local_facts(
    script: IRScript,
    *,
    caller_qname: str,
    known_procs: set[str],
    facts: _LocalFacts,
) -> None:
    for stmt in script.statements:
        if isinstance(stmt, IRBarrier):
            facts.has_barrier = True
            facts.effect_writes |= EffectRegion.UNKNOWN_STATE
            # An ``IRBarrier`` retains the original ``command`` / ``args``
            # of the command that crossed the analysis boundary (e.g.
            # ``eval`` or a user-stubbed wrapper like ``db_eval``).  If
            # any of those args carry ``ArgRole.BODY``, scan them so
            # callees inside the script body still register as call-graph
            # edges.
            if stmt.command and stmt.args:
                body_indices = arg_indices_for_role(stmt.command, list(stmt.args), ArgRole.BODY)
                for idx in body_indices:
                    if 0 <= idx < len(stmt.args):
                        raw = stmt.args[idx]
                        head = raw.lstrip()[:1]
                        if head in ("$", "["):
                            continue
                        body_text = _strip_braces(raw)
                        _scan_script_text(
                            body_text,
                            caller_qname=caller_qname,
                            known_procs=known_procs,
                            facts=facts,
                        )
            continue

        # Barrier-relaxed ``uplevel`` / ``eval`` preserve barrier
        # semantics for interprocedural analysis — the body may
        # clobber arbitrary state, same as the classic ``uplevel``
        # / ``eval`` IRBarrier it replaces.
        from compiler.ir import IRBlock as _IRBlock
        from compiler.ir import IRUpFrame as _IRUpFrame

        if isinstance(stmt, _IRUpFrame) or (
            isinstance(stmt, _IRBlock)
            and stmt.source_tokens is not None
            and stmt.source_tokens.argv_texts
            and stmt.source_tokens.argv_texts[0] == "eval"
        ):
            facts.has_barrier = True
            facts.effect_writes |= EffectRegion.UNKNOWN_STATE
            continue

        if isinstance(stmt, IRCall):
            # Track scope-aliasing declarations (``global`` / ``variable`` /
            # ``upvar #0``) so a later bare ``set g`` / ``incr g`` to an
            # aliased name is recognised as a global write.  These declaration
            # commands record ``defs`` for the alias itself, so handle them
            # before the defs-based write check below (declaring is not
            # writing a value).
            alias_names = _global_alias_names(stmt.command, stmt.args)
            if alias_names is not None:
                if "" in alias_names:
                    # Dynamic/unbounded alias target — be conservative.
                    facts.writes_global = True
                facts.global_aliases |= {n for n in alias_names if n}
            else:
                # A non-declaration command that writes a global-aliased or
                # ``::``-qualified variable (``append g``, ``lappend g``,
                # ``dict set ::cfg ...``) mutates caller-visible state.
                for name in stmt.defs:
                    if name and (name.startswith("::") or name in facts.global_aliases):
                        facts.writes_global = True
                        break

            target = resolve_call_target(stmt.command, stmt.args, caller_qname, known_procs)
            if target is None:
                _apply_effect(
                    facts,
                    classify_side_effects(stmt.command, stmt.args),
                )
            else:
                facts.calls.add(target)
            # Recurse into ``ArgRole.BODY`` arguments of the call so
            # callees inside the script body of commands like ``catch``,
            # ``eval``, or user-stubbed wrappers (``db_eval $sql $script``)
            # show up as call-graph edges.  Skip BODY args that are not
            # literal scripts (``$var``, ``[cmd]``) since their contents
            # are not statically known.
            body_indices = arg_indices_for_role(stmt.command, list(stmt.args), ArgRole.BODY)
            for idx in body_indices:
                if 0 <= idx < len(stmt.args):
                    raw = stmt.args[idx]
                    head = raw.lstrip()[:1]
                    if head in ("$", "["):
                        continue
                    body_text = _strip_braces(raw)
                    _scan_script_text(
                        body_text,
                        caller_qname=caller_qname,
                        known_procs=known_procs,
                        facts=facts,
                    )
            continue

        if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
            if stmt.name.startswith("::") or stmt.name in facts.global_aliases:
                facts.writes_global = True
            if isinstance(stmt, IRAssignExpr) and _expr_has_command_sub(stmt.expr):
                facts.has_unknown_calls = True
            if isinstance(stmt, IRAssignValue) and _contains_command_substitution(stmt.value):
                _scan_embedded_commands(
                    stmt.value,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            if isinstance(stmt, IRIncr) and _contains_command_substitution(stmt.amount):
                _scan_embedded_commands(
                    stmt.amount or "",
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            continue

        if isinstance(stmt, IRReturn):
            if stmt.value and "[" in stmt.value:
                _scan_embedded_commands(
                    stmt.value,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            continue

        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                _scan_expr_for_calls(
                    clause.condition,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
                _scan_local_facts(
                    clause.body,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            if stmt.else_body is not None:
                _scan_local_facts(
                    stmt.else_body,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            continue

        if isinstance(stmt, IRFor):
            _scan_local_facts(
                stmt.init,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            _scan_local_facts(
                stmt.body,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            _scan_local_facts(
                stmt.next,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            _scan_expr_for_calls(
                stmt.condition,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            if _expr_has_command_sub(stmt.condition):
                facts.has_unknown_calls = True
            continue

        if isinstance(stmt, IRSwitch):
            if _contains_command_substitution(stmt.subject):
                _scan_embedded_commands(
                    stmt.subject,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            for arm in stmt.arms:
                if arm.body is not None:
                    _scan_local_facts(
                        arm.body,
                        caller_qname=caller_qname,
                        known_procs=known_procs,
                        facts=facts,
                    )
            if stmt.default_body is not None:
                _scan_local_facts(
                    stmt.default_body,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            continue

        if isinstance(stmt, IRWhile):
            _scan_local_facts(
                stmt.body,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            _scan_expr_for_calls(
                stmt.condition,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            if _expr_has_command_sub(stmt.condition):
                facts.has_unknown_calls = True
            continue

        if isinstance(stmt, IRForeach):
            _scan_local_facts(
                stmt.body,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            continue

        if isinstance(stmt, IRCatch):
            _scan_local_facts(
                stmt.body,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            continue

        if isinstance(stmt, IRTry):
            _scan_local_facts(
                stmt.body,
                caller_qname=caller_qname,
                known_procs=known_procs,
                facts=facts,
            )
            for handler in stmt.handlers:
                _scan_local_facts(
                    handler.body,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )
            if stmt.finally_body is not None:
                _scan_local_facts(
                    stmt.finally_body,
                    caller_qname=caller_qname,
                    known_procs=known_procs,
                    facts=facts,
                )


def _compute_param_dependencies(
    cfg: CFGFunction,
    ssa: SSAFunction,
    params: tuple[str, ...],
) -> dict[SSAValueKey, frozenset[str]]:
    param_set = set(params)
    deps: dict[SSAValueKey, set[str]] = {}
    order = cfg.reverse_postorder()

    changed = True
    while changed:
        changed = False

        for bn in order:
            block = ssa.blocks[bn]

            for phi in block.phis:
                union: set[str] = set()
                for incoming_ver in phi.incoming.values():
                    if incoming_ver <= 0:
                        if phi.name in param_set:
                            union.add(phi.name)
                        continue
                    union |= deps.get((phi.name, incoming_ver), set())
                key = (phi.name, phi.version)
                old = deps.get(key, set())
                new = old | union
                if new != old:
                    deps[key] = new
                    changed = True

            for stmt in block.statements:
                use_union: set[str] = set()
                for use_name, use_ver in stmt.uses.items():
                    if use_ver <= 0:
                        if use_name in param_set:
                            use_union.add(use_name)
                        continue
                    use_union |= deps.get((use_name, use_ver), set())

                for def_name, def_ver in stmt.defs.items():
                    key = (def_name, def_ver)
                    old = deps.get(key, set())
                    new = old | use_union
                    if new != old:
                        deps[key] = new
                        changed = True

    return {k: frozenset(v) for k, v in deps.items()}


def _collect_return_infos(
    cfg: CFGFunction,
    ssa: SSAFunction,
    values: dict[SSAValueKey, LatticeValue],
    param_deps: dict[SSAValueKey, frozenset[str]],
    params: tuple[str, ...],
    unreachable_blocks: set[str],
) -> tuple[_ReturnInfo, ...]:
    param_set = set(params)
    infos: list[_ReturnInfo] = []
    reachable_blocks = set(cfg.blocks) - set(unreachable_blocks)

    for bn in sorted(reachable_blocks):
        block = cfg.blocks[bn]
        term = block.terminator
        if not isinstance(term, CFGReturn):
            continue

        ret = term.value
        ssa_block = ssa.blocks.get(bn)
        if ret is None or ssa_block is None:
            infos.append(
                _ReturnInfo(
                    const_known=False,
                    const_value=None,
                    param_deps=frozenset(),
                    passthrough_param=None,
                )
            )
            continue

        deps: set[str] = set()
        for name in _vars_in_word(ret):
            ver = ssa_block.exit_versions.get(name, 0)
            if ver <= 0:
                if name in param_set:
                    deps.add(name)
                continue
            deps |= set(param_deps.get((name, ver), frozenset()))

        const_known = False
        const_value: int | float | bool | str | None = None
        literal = _parse_literal_word(ret)
        if literal is not None:
            const_known = True
            const_value = literal
        else:
            vname = _single_simple_var_word(ret)
            if vname is not None:
                ver = ssa_block.exit_versions.get(vname, 0)
                if ver > 0:
                    lv = values.get((vname, ver), LatticeValue.unknown())
                    if lv.kind is LatticeKind.CONST:
                        const_known = True
                        const_value = lv.value

        passthrough_param: str | None = None
        vname = _single_simple_var_word(ret)
        if vname is not None and vname in param_set:
            # ver<=0 means the return directly references the input parameter.
            ver = ssa_block.exit_versions.get(vname, 0)
            if ver <= 0:
                passthrough_param = vname

        infos.append(
            _ReturnInfo(
                const_known=const_known,
                const_value=const_value,
                param_deps=frozenset(deps),
                passthrough_param=passthrough_param,
            )
        )

    return tuple(infos)


def _return_summary(
    infos: tuple[_ReturnInfo, ...],
) -> tuple[bool, int | float | bool | str | None, tuple[str, ...], str | None]:
    if not infos:
        return False, None, (), None

    param_deps = tuple(sorted({p for info in infos for p in info.param_deps}))

    if all(info.const_known for info in infos):
        first = infos[0].const_value
        if all(info.const_value == first for info in infos[1:]):
            returns_constant = True
            constant_return = first
        else:
            returns_constant = False
            constant_return = None
    else:
        returns_constant = False
        constant_return = None

    passthrough: str | None = None
    candidates = {info.passthrough_param for info in infos}
    if len(candidates) == 1:
        only = next(iter(candidates))
        if only is not None:
            passthrough = only

    return returns_constant, constant_return, param_deps, passthrough


def _arity_for_params(params: tuple[str, ...]) -> Arity:
    if params and params[-1] == "args":
        return Arity(len(params) - 1)
    return Arity(len(params), len(params))


def _arity_matches(summary: ProcSummary, nargs: int) -> bool:
    return summary.arity.accepts(nargs)


def _cache_key_for_proc(
    source: str,
    qname: str,
    proc: IRProcedure,
    stub_fingerprint: int = 0,
    context_fingerprint: int = 0,
) -> tuple[str, int] | None:
    """Return cache key for a proc based on its source slice.

    The fingerprint covers the active stub signature overlay because
    summaries depend on how role-aware command lookups resolve
    ``ArgRole.BODY`` / ``ArgRole.EXPR`` — adding or changing a stub
    must invalidate the cached entry even when the proc body text is
    unchanged.

    ``context_fingerprint`` covers the module-global CFG-construction context
    (see :func:`compiler.cfg.cfg_context_fingerprint`): the local summary is
    derived from the context-built CFG/SSA/analysis, so a callee's upvar change
    must invalidate a caller whose own text is unchanged.
    """
    start = proc.range.start.offset
    end = proc.range.end.offset
    if start < 0 or end < start or end > len(source):
        return None
    # range.end.offset is the proc's last character (inclusive), so slice
    # end-exclusive at end+1 — otherwise a same-length edit to the final
    # character of the proc body leaves the hash unchanged and the cached
    # summary is wrongly reused.
    return (qname, hash((source[start : end + 1], stub_fingerprint, context_fingerprint)))


def _summarise_proc_local(
    qname: str,
    proc: IRProcedure | IRMethodDef,
    *,
    known: set[str],
    cfg: CFGFunction,
    ssa: SSAFunction,
    analysis: FunctionAnalysis,
) -> ProcLocalSummary:
    """Compute local (non-transitive) summary facts for one procedure.

    Accepts an ``IRMethodDef`` too (SF-2): only ``.body`` and ``.params``
    are read, both of which the method form carries.
    """
    facts = _LocalFacts(
        calls=set(),
        has_barrier=False,
        has_unknown_calls=False,
        writes_global=False,
        effect_reads=EffectRegion.NONE,
        effect_writes=EffectRegion.NONE,
    )
    _scan_local_facts(proc.body, caller_qname=qname, known_procs=known, facts=facts)

    # Complexity guard: for a pathologically large (generated) body, deep
    # analysis is skipped and SSA is trivial (empty blocks) while the CFG still
    # has blocks — the param-dependency + return-summary passes below would
    # crash on the empty SSA and/or iterate forever.  Return a conservative
    # opaque summary (impure: barrier + unknown-calls + writes-global, no
    # param-deps / constant return); the call graph + effect facts from the IR
    # scan above are kept.  Key on trivial SSA, not block count alone, so a
    # byte-heavy *block-light* body (guarded before SSA via ``force_guard``) is
    # handled the same as a block-heavy one.
    if is_complexity_guarded(cfg) or (cfg.blocks and not ssa.blocks):
        return ProcLocalSummary(
            qualified_name=qname,
            params=proc.params,
            arity=_arity_for_params(proc.params),
            calls=tuple(sorted(facts.calls)),
            has_barrier=True,
            has_unknown_calls=True,
            writes_global=True,
            local_effect_reads=facts.effect_reads,
            local_effect_writes=facts.effect_writes,
            returns_constant=False,
            constant_return=None,
            return_depends_on_params=(),
            return_passthrough_param=None,
        )

    # Second pass: the CFG builder augments IRCall.defs with caller-side
    # variable names from upvar procs.  Check these for global writes that
    # the raw-IR walk above cannot see.
    if not facts.writes_global:
        for block in cfg.blocks.values():
            for stmt in block.statements:
                if isinstance(stmt, IRCall) and stmt.defs:
                    if any(d.startswith("::") for d in stmt.defs):
                        facts.writes_global = True
                        break
            if facts.writes_global:
                break

    param_deps = _compute_param_dependencies(cfg, ssa, proc.params)
    return_infos = _collect_return_infos(
        cfg,
        ssa,
        analysis.values,
        param_deps,
        proc.params,
        analysis.unreachable_blocks,
    )
    returns_constant, constant_return, dep_params, passthrough = _return_summary(return_infos)
    arity = _arity_for_params(proc.params)

    return ProcLocalSummary(
        qualified_name=qname,
        params=proc.params,
        arity=arity,
        calls=tuple(sorted(facts.calls)),
        has_barrier=facts.has_barrier,
        has_unknown_calls=facts.has_unknown_calls,
        writes_global=facts.writes_global,
        local_effect_reads=facts.effect_reads,
        local_effect_writes=facts.effect_writes,
        returns_constant=returns_constant,
        constant_return=constant_return,
        return_depends_on_params=dep_params,
        return_passthrough_param=passthrough,
    )


def analyse_interprocedural_ir(
    ir_module: IRModule,
    *,
    proc_units: dict[str, tuple[CFGFunction, SSAFunction, FunctionAnalysis]] | None = None,
    method_units: dict[str, tuple[CFGFunction, SSAFunction, FunctionAnalysis]] | None = None,
    source: str | None = None,
    proc_local_cache: dict[tuple[str, int], ProcLocalSummary] | None = None,
    prune_local_cache: bool = True,
    stub_fingerprint: int = 0,
    context_fingerprint: int = 0,
    deep_param_traits: bool = False,
) -> InterproceduralAnalysis:
    """Build conservative per-procedure summaries from lowered IR.

    When *proc_units* is provided (mapping qualified name to
    ``(cfg, ssa, analysis)``), the per-procedure pipeline is skipped —
    the pre-built artefacts are used directly.

    When *method_units* is provided, TclOO method bodies (in
    ``ir_module.methods``) are summarised into ``MethodSummary`` entries
    on the returned ``InterproceduralAnalysis.methods`` (SF-2 — consumed
    by the O126 ``my <method>`` purity gate).

    When *source* and *proc_local_cache* are provided, local summary
    facts are reused for unchanged procedures using key
    ``(qualified_name, hash(proc_source_text))``.
    """
    if not ir_module.procedures and not ir_module.methods:
        return InterproceduralAnalysis(procedures={})

    known = set(ir_module.procedures.keys())
    local_proc_summaries: dict[str, ProcLocalSummary] = {}
    active_cache_keys: set[tuple[str, int]] = set()

    if proc_units is None:
        cfg_module = build_cfg(ir_module)

    for qname, proc in ir_module.procedures.items():
        cache_key: tuple[str, int] | None = None
        if source is not None and proc_local_cache is not None:
            cache_key = _cache_key_for_proc(
                source, qname, proc, stub_fingerprint, context_fingerprint
            )
            if cache_key is not None:
                cached_local = proc_local_cache.get(cache_key)
                if cached_local is not None and cached_local.params == proc.params:
                    local_proc_summaries[qname] = cached_local
                    active_cache_keys.add(cache_key)
                    continue

        if proc_units is not None:
            cfg, ssa, analysis = proc_units[qname]
        else:
            cfg = cfg_module.procedures[qname]
            ssa = build_ssa(cfg)
            analysis = analyse_function(cfg, ssa)

        local_summary = _summarise_proc_local(
            qname,
            proc,
            known=known,
            cfg=cfg,
            ssa=ssa,
            analysis=analysis,
        )
        local_proc_summaries[qname] = local_summary

        if cache_key is not None and proc_local_cache is not None:
            proc_local_cache[cache_key] = local_summary
            active_cache_keys.add(cache_key)

    if proc_local_cache is not None and source is not None and prune_local_cache:
        stale_keys = [key for key in proc_local_cache if key not in active_cache_keys]
        for key in stale_keys:
            del proc_local_cache[key]

    local_pure_base: dict[str, bool] = {}
    local_reads: dict[str, EffectRegion] = {}
    local_writes: dict[str, EffectRegion] = {}
    for qname, local in local_proc_summaries.items():
        local_reads[qname] = local.local_effect_reads
        local_writes[qname] = local.local_effect_writes
        local_pure_base[qname] = (
            not local.has_barrier
            and not local.has_unknown_calls
            and not local.writes_global
            and local.local_effect_writes == EffectRegion.NONE
        )

    # Reverse call graph: callers[callee] = procs that call it.  Both
    # fixpoints below propagate a monotone change at a proc only to its
    # callers, so a worklist visits O(edges) work instead of re-scanning
    # every proc each pass (round-robin was O(procs² · fanout) on deep
    # call chains).
    callers: dict[str, set[str]] = {qname: set() for qname in local_proc_summaries}
    for qname, local in local_proc_summaries.items():
        for callee in local.calls:
            if callee in callers:
                callers[callee].add(qname)

    pure: dict[str, bool] = dict(local_pure_base)
    worklist: list[str] = list(local_proc_summaries)
    queued: set[str] = set(worklist)
    while worklist:
        qname = worklist.pop()
        queued.discard(qname)
        local = local_proc_summaries[qname]
        new_pure = local_pure_base[qname] and all(pure.get(callee, False) for callee in local.calls)
        if new_pure != pure[qname]:
            pure[qname] = new_pure
            for caller in callers[qname]:
                if caller not in queued:
                    worklist.append(caller)
                    queued.add(caller)

    effect_reads: dict[str, EffectRegion] = dict(local_reads)
    effect_writes: dict[str, EffectRegion] = dict(local_writes)
    worklist = list(local_proc_summaries)
    queued = set(worklist)
    while worklist:
        qname = worklist.pop()
        queued.discard(qname)
        local = local_proc_summaries[qname]
        new_reads = local_reads[qname]
        new_writes = local_writes[qname]
        for callee in local.calls:
            new_reads |= effect_reads.get(callee, EffectRegion.UNKNOWN_STATE)
            new_writes |= effect_writes.get(callee, EffectRegion.UNKNOWN_STATE)
        dirty = False
        if new_reads != effect_reads[qname]:
            effect_reads[qname] = new_reads
            dirty = True
        if new_writes != effect_writes[qname]:
            effect_writes[qname] = new_writes
            dirty = True
        if dirty:
            for caller in callers[qname]:
                if caller not in queued:
                    worklist.append(caller)
                    queued.add(caller)

    summaries: dict[str, ProcSummary] = {}
    for qname in sorted(ir_module.procedures):
        local = local_proc_summaries[qname]
        is_pure = pure[qname]
        can_fold = is_pure and (
            local.returns_constant or local.return_passthrough_param is not None
        )
        if can_fold and qname in ir_module.redefined_procedures:
            can_fold = False

        # Infer proc argument traits from body source.  LSP-synchronous
        # callers stay on the shallow pass; offline analytics paths
        # (``tcl callgraph``, the compiler explorer, the MCP server)
        # opt into the deep pass to descend into nested script bodies
        # (e.g. a param that only reaches an ``eval`` from inside a
        # ``foreach`` body).  Deep results are merged with shallow
        # rather than replacing them so neither pass loses signal.
        proc = ir_module.procedures[qname]
        traits: dict[str, frozenset[ProcArgTrait]] = {}
        if proc.body_source and local.params:
            try:
                traits = infer_param_traits(local.params, proc.body_source)
                if deep_param_traits:
                    deep = infer_param_traits_deep(local.params, proc.body_source)
                    traits = merge_traits(traits, deep)
            except Exception:
                log.debug("trait inference failed for %s", qname, exc_info=True)

        summaries[qname] = ProcSummary(
            qualified_name=qname,
            params=local.params,
            arity=local.arity,
            calls=local.calls,
            has_barrier=local.has_barrier,
            has_unknown_calls=local.has_unknown_calls,
            writes_global=local.writes_global,
            pure=is_pure,
            effect_reads=effect_reads[qname],
            effect_writes=effect_writes[qname],
            returns_constant=local.returns_constant,
            constant_return=local.constant_return,
            return_depends_on_params=local.return_depends_on_params,
            return_passthrough_param=local.return_passthrough_param,
            can_fold_static_calls=can_fold,
            param_traits=traits,
        )

    # SF-2: summarise TclOO method bodies into MethodSummary entries.  The
    # O126 ``my <method>`` purity gate only consults ``summary.pure``, so the
    # purity rule is intentionally conservative: a method is pure iff its own
    # body has no observable side effect AND every *proc* it calls is pure.
    # A ``my <other_method>`` / ``next`` dispatch surfaces as an unknown call
    # (``has_unknown_calls``), which already forces the method impure — so we
    # never mark a method pure on the strength of an unproven peer method
    # (sound: false negatives only, the optimiser stays conservative).
    method_summaries: dict[str, MethodSummary] = {}
    if ir_module.methods:
        method_known = known | set(ir_module.methods.keys())
        for mqname, ir_method in ir_module.methods.items():
            if method_units is not None and mqname in method_units:
                m_cfg, m_ssa, m_analysis = method_units[mqname]
            elif proc_units is None:
                # No pre-built units and no method CFGs available; skip.
                continue
            else:
                continue
            m_local = _summarise_proc_local(
                mqname,
                ir_method,
                known=method_known,
                cfg=m_cfg,
                ssa=m_ssa,
                analysis=m_analysis,
            )
            # Instance-variable writes mutate object state and survive the
            # method call — so a method that writes any in-scope instance var
            # is impure even though the write looks like a plain local
            # ``set`` (sound: never deletes a state-changing self-dispatch).
            written_ivars: set[str] = set()
            if ir_method.instance_vars:
                for block in m_cfg.blocks.values():
                    for stmt in block.statements:
                        if isinstance(stmt, IRCall):
                            # ``variable`` / ``upvar`` link or declare a name;
                            # they are not writes to instance state.  Counting
                            # their defs would mark a read-only instance-var
                            # method (``variable x; return $x``) impure.
                            if stmt.canonical_command in ("::variable", "::upvar"):
                                continue
                            written = stmt.defs
                        else:
                            nm = getattr(stmt, "name", None)
                            written = (nm,) if isinstance(nm, str) else ()
                        for raw in written:
                            if not raw:
                                continue
                            # The def name may be an array element
                            # (``counter(0)``) or qualified; compare the base
                            # scalar/array name against the declared instance
                            # vars so element writes are not missed.
                            base = _split_array_name(_normalise_var_name(raw))[0]
                            if base in ir_method.instance_vars:
                                written_ivars.add(base)
            m_pure_base = (
                not m_local.has_barrier
                and not m_local.has_unknown_calls
                and not m_local.writes_global
                and not written_ivars
                and m_local.local_effect_writes == EffectRegion.NONE
            )
            m_pure = m_pure_base and all(pure.get(callee, False) for callee in m_local.calls)
            # A redefined method (later oo::define / duplicate body) is
            # conservatively impure: the stored body may not be the one a
            # given dispatch runs, so the O126 my-dispatch gate must not
            # delete on its proven purity.
            if mqname in ir_module.redefined_methods:
                m_pure = False
            m_effect_reads = m_local.local_effect_reads
            m_effect_writes = m_local.local_effect_writes
            for callee in m_local.calls:
                m_effect_reads |= effect_reads.get(callee, EffectRegion.NONE)
                m_effect_writes |= effect_writes.get(callee, EffectRegion.NONE)
            method_summaries[mqname] = MethodSummary(
                qualified_name=mqname,
                params=m_local.params,
                arity=m_local.arity,
                calls=m_local.calls,
                has_barrier=m_local.has_barrier,
                has_unknown_calls=m_local.has_unknown_calls,
                writes_global=m_local.writes_global,
                pure=m_pure,
                effect_reads=m_effect_reads,
                effect_writes=m_effect_writes,
                returns_constant=m_local.returns_constant,
                constant_return=m_local.constant_return,
                return_depends_on_params=m_local.return_depends_on_params,
                return_passthrough_param=m_local.return_passthrough_param,
                can_fold_static_calls=False,
                class_name=ir_method.class_name,
                method_kind=ir_method.kind,
                writes_instance_vars=frozenset(written_ivars),
            )

    return InterproceduralAnalysis(procedures=summaries, methods=method_summaries)


def analyse_interprocedural_source(source: str) -> InterproceduralAnalysis:
    """Lower source then build conservative interprocedural summaries."""
    return analyse_interprocedural_ir(lower_to_ir(source))


def _try_fold_return_value(
    ret: str,
    exit_versions: dict[str, int],
    values: dict[tuple[str, int], LatticeValue],
) -> int | float | bool | str | None:
    """Try to fold a CFGReturn value to a constant.

    Handles literals, simple variable references, string interpolation,
    and ``[expr {...}]`` command substitutions.
    """
    # 1) Literal
    literal = _parse_literal_word(ret)
    if literal is not None:
        return literal

    # 2) Simple variable reference
    vname = _single_simple_var_word(ret)
    if vname is not None:
        ver = exit_versions.get(vname, 0)
        lv = values.get((vname, ver), LatticeValue.unknown())
        if lv.kind is LatticeKind.CONST:
            return lv.value
        return None

    # 3) Tokenise for interpolation and command substitution
    # Build env from ALL CONST values in the values dict (including seeded
    # version-0 params), not just exit_versions which may be empty.
    env: dict[str, int | float | bool | str] = {}
    for (name, ver), lv in values.items():
        if lv.kind is LatticeKind.CONST and lv.value is not None:
            if name not in env or ver > 0:
                env[name] = lv.value
    # Overlay with exit_versions for precise block-local state
    for name, ver in exit_versions.items():
        lv = values.get((name, ver), LatticeValue.unknown())
        if lv.kind is LatticeKind.CONST and lv.value is not None:
            env[name] = lv.value

    pieces: list[str] = []
    lex_tokens, _ = tokenise(ret, 0, 0, 0)
    for tok in lex_tokens:
        if tok.type is TokenType.VAR:
            name = _normalise_var_name(tok.text)
            if name not in env:
                return None
            pieces.append(str(env[name]))
        elif tok.type is TokenType.CMD:
            # Only handle [expr {...}]
            cmd_text = tok.text.strip()
            if not cmd_text.startswith("expr"):
                return None
            parts = cmd_text.split(None, 1)
            if len(parts) != 2 or parts[0] != "expr":
                return None
            expr_arg = parts[1].strip()
            if expr_arg.startswith("{") and expr_arg.endswith("}"):
                expr_arg = expr_arg[1:-1]
            result = _evaluate_expr(expr_arg, env)
            if result is None:
                return None
            pieces.append(str(result))
        else:
            pieces.append(tok.text)
    result_str = "".join(pieces)
    # Try to return as int if possible
    if _DECIMAL_INT_RE.fullmatch(result_str.strip()):
        try:
            return int(result_str.strip())
        except ValueError:
            pass
    return result_str


def _resolve_return_constant(
    cfg: CFGFunction,
    ssa: SSAFunction,
    values: dict[tuple[str, int], LatticeValue],
    unreachable_blocks: set[str],
) -> int | float | bool | str | None:
    """Check all reachable CFGReturn terminators; return the constant if all agree."""
    reachable = set(cfg.blocks) - unreachable_blocks
    results: list[int | float | bool | str] = []
    for bn in sorted(reachable):
        term = cfg.blocks[bn].terminator
        if not isinstance(term, CFGReturn):
            continue
        if term.value is None:
            return None  # void return
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            return None
        folded = _try_fold_return_value(term.value, ssa_block.exit_versions, values)
        if folded is None:
            return None
        results.append(folded)
    if not results:
        return None
    first = results[0]
    return first if all(r == first for r in results) else None


def evaluate_proc_with_constants(
    cfg: CFGFunction,
    params: tuple[str, ...],
    args: tuple[int | bool | str, ...],
) -> int | float | bool | str | None:
    """Re-analyse a procedure with parameters bound to constants.

    Returns the constant return value if determinable, else None.
    """
    if len(args) != len(params):
        return None
    param_constants = {(p, 0): LatticeValue.const(a) for p, a in zip(params, args)}
    ssa = build_ssa(cfg)
    analysis = analyse_function(cfg, ssa, param_constants=param_constants)
    return _resolve_return_constant(cfg, ssa, analysis.values, analysis.unreachable_blocks)


def fold_static_proc_call(
    analysis: InterproceduralAnalysis,
    proc_name: str,
    args: tuple[int | bool | str, ...],
) -> int | float | bool | str | None:
    """Fold a proc call to a constant when summary guarantees safety."""
    qname = _normalise_qualified_name(proc_name)
    summary = analysis.procedures.get(qname)
    if summary is None:
        return None
    if not summary.can_fold_static_calls:
        return None
    if not _arity_matches(summary, len(args)):
        return None

    if summary.returns_constant:
        return summary.constant_return

    if summary.return_passthrough_param is not None:
        try:
            idx = summary.params.index(summary.return_passthrough_param)
        except ValueError:
            return None
        if 0 <= idx < len(args):
            return args[idx]

    return None
