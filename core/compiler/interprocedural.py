"""Interprocedural procedure summaries (Phase 5).

Conservative summaries are built per lowered proc to describe:
- side-effect / purity shape
- internal call graph edges
- constant return behaviour
- parameter sensitivity of return values
- safe static call folding opportunities
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ..commands.registry.signatures import Arity
from ..common.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from ..common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ..parsing.lexer import TclLexer
from ..parsing.tokens import TokenType
from .cfg import CFGFunction, CFGReturn, build_cfg
from .core_analyses import FunctionAnalysis, LatticeKind, LatticeValue, analyse_function
from .core_analyses import _expr_has_command as _expr_has_command_sub
from .eval_helpers import DECIMAL_INT_RE as _DECIMAL_INT_RE
from .ir import (
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
    IRModule,
    IRProcedure,
    IRReturn,
    IRScript,
    IRSwitch,
    IRTry,
    IRWhile,
)
from .lowering import lower_to_ir
from .side_effects import EffectRegion, classify_side_effects
from .ssa import SSAFunction, SSAValueKey, build_ssa
from .static_loops import evaluate_expr_with_constants as _evaluate_expr
from .var_refs import VarReferenceScanner, VarScanOptions

_SIMPLE_VAR_WORD_RE = re.compile(r"\$(?:\{[A-Za-z_][A-Za-z0-9_:]*\}|[A-Za-z_][A-Za-z0-9_:]*)\Z")
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


@dataclass(frozen=True, slots=True)
class InterproceduralAnalysis:
    procedures: dict[str, ProcSummary]


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
    """
    if command == "call" and args:
        return resolve_internal_call(args[0], caller_qname, known)
    return resolve_internal_call(command, caller_qname, known)


def _vars_in_script(source: str) -> set[str]:
    return _VAR_REF_SCANNER.scan_script(source)


def _vars_in_word(text: str) -> set[str]:
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
    if not _SIMPLE_VAR_WORD_RE.fullmatch(stripped):
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


def _scan_embedded_commands(
    text: str,
    *,
    caller_qname: str,
    known_procs: set[str],
    facts: _LocalFacts,
) -> None:
    lexer2 = TclLexer(text)
    while True:
        tok2 = lexer2.get_token()
        if tok2 is None:
            break
        if tok2.type is not TokenType.CMD:
            continue
        cmd_text = tok2.text.strip()
        if not cmd_text:
            continue
        parts = cmd_text.split()
        if not parts:
            continue
        cmd_word = parts[0]
        cmd_args = tuple(parts[1:])
        target = resolve_call_target(cmd_word, cmd_args, caller_qname, known_procs)
        if target is not None:
            facts.calls.add(target)
        else:
            _apply_effect(facts, classify_side_effects(cmd_word, cmd_args))


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
            continue

        if isinstance(stmt, IRCall):
            target = resolve_call_target(stmt.command, stmt.args, caller_qname, known_procs)
            if target is None:
                _apply_effect(
                    facts,
                    classify_side_effects(stmt.command, stmt.args),
                )
            else:
                facts.calls.add(target)
            continue

        if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
            if stmt.name.startswith("::"):
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
            if _expr_has_command_sub(stmt.condition):
                facts.has_unknown_calls = True
            continue

        if isinstance(stmt, IRSwitch):
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
    ssa: SSAFunction,
    params: tuple[str, ...],
) -> dict[SSAValueKey, frozenset[str]]:
    param_set = set(params)
    deps: dict[SSAValueKey, set[str]] = {}
    order = list(ssa.blocks.keys())

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
) -> tuple[str, int] | None:
    """Return cache key for a proc based on its source slice."""
    start = proc.range.start.offset
    end = proc.range.end.offset
    if start < 0 or end < start or end > len(source):
        return None
    return (qname, hash(source[start:end]))


def _summarise_proc_local(
    qname: str,
    proc: IRProcedure,
    *,
    known: set[str],
    cfg: CFGFunction,
    ssa: SSAFunction,
    analysis: FunctionAnalysis,
) -> ProcLocalSummary:
    """Compute local (non-transitive) summary facts for one procedure."""
    facts = _LocalFacts(
        calls=set(),
        has_barrier=False,
        has_unknown_calls=False,
        writes_global=False,
        effect_reads=EffectRegion.NONE,
        effect_writes=EffectRegion.NONE,
    )
    _scan_local_facts(proc.body, caller_qname=qname, known_procs=known, facts=facts)

    param_deps = _compute_param_dependencies(ssa, proc.params)
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
    source: str | None = None,
    proc_local_cache: dict[tuple[str, int], ProcLocalSummary] | None = None,
    prune_local_cache: bool = True,
) -> InterproceduralAnalysis:
    """Build conservative per-procedure summaries from lowered IR.

    When *proc_units* is provided (mapping qualified name to
    ``(cfg, ssa, analysis)``), the per-procedure pipeline is skipped —
    the pre-built artefacts are used directly.

    When *source* and *proc_local_cache* are provided, local summary
    facts are reused for unchanged procedures using key
    ``(qualified_name, hash(proc_source_text))``.
    """
    if not ir_module.procedures:
        return InterproceduralAnalysis(procedures={})

    known = set(ir_module.procedures.keys())
    local_proc_summaries: dict[str, ProcLocalSummary] = {}
    active_cache_keys: set[tuple[str, int]] = set()

    if proc_units is None:
        cfg_module = build_cfg(ir_module)

    for qname, proc in ir_module.procedures.items():
        cache_key: tuple[str, int] | None = None
        if source is not None and proc_local_cache is not None:
            cache_key = _cache_key_for_proc(source, qname, proc)
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

    pure: dict[str, bool] = dict(local_pure_base)
    changed = True
    while changed:
        changed = False
        for qname, local in local_proc_summaries.items():
            new_pure = local_pure_base[qname] and all(
                pure.get(callee, False) for callee in local.calls
            )
            if new_pure != pure[qname]:
                pure[qname] = new_pure
                changed = True

    effect_reads: dict[str, EffectRegion] = dict(local_reads)
    effect_writes: dict[str, EffectRegion] = dict(local_writes)
    changed = True
    while changed:
        changed = False
        for qname, local in local_proc_summaries.items():
            new_reads = local_reads[qname]
            new_writes = local_writes[qname]
            for callee in local.calls:
                new_reads |= effect_reads.get(callee, EffectRegion.UNKNOWN_STATE)
                new_writes |= effect_writes.get(callee, EffectRegion.UNKNOWN_STATE)
            if new_reads != effect_reads[qname]:
                effect_reads[qname] = new_reads
                changed = True
            if new_writes != effect_writes[qname]:
                effect_writes[qname] = new_writes
                changed = True

    summaries: dict[str, ProcSummary] = {}
    for qname in sorted(ir_module.procedures):
        local = local_proc_summaries[qname]
        is_pure = pure[qname]
        can_fold = is_pure and (
            local.returns_constant or local.return_passthrough_param is not None
        )
        if can_fold and qname in ir_module.redefined_procedures:
            can_fold = False

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
        )

    return InterproceduralAnalysis(procedures=summaries)


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

    # 3) Tokenise with TclLexer for interpolation and command substitution
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
    lexer = TclLexer(ret)
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
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
