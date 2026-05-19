"""Analyse conf-wrapped iRules files (``ltm rule`` / ``gtm rule`` stanzas).

Extracts each rule body, analyses it independently at depth 0, then
merges results with range-shifted positions so diagnostics, symbols,
and scopes map back to the original file coordinates.  All rules in
a single file share procs and RULE_INIT variables (flat sharing).
"""

from __future__ import annotations

from compiler.parsing.tokens import SourcePosition
from dialects.f5.bigip.rule_extract import EmbeddedRule, find_embedded_rules
from shared.document_buffer import DocumentBuffer

from . import Analyser
from .semantic_model import (
    AnalysisResult,
    ClassDef,
    CodeFix,
    CommandInvocation,
    Diagnostic,
    MethodDef,
    PackageProvide,
    PackageRequire,
    ProcDef,
    PropertyDef,
    Range,
    RegexPattern,
    Scope,
    SourceTarget,
    StubCommandDef,
    StubExprDef,
    VarDef,
)


def _shift_position(
    pos: SourcePosition, base_line: int, base_char: int, base_offset: int
) -> SourcePosition:
    """Shift *pos* by a base line/character/offset.

    For line 0, column is shifted by *base_char*; subsequent lines
    only shift by *base_line* (their column is relative to the body,
    which already starts at column 0 on its own line).
    The byte offset is always shifted by *base_offset*.
    """
    if pos.line == 0:
        return SourcePosition(
            line=pos.line + base_line,
            character=pos.character + base_char,
            offset=pos.offset + base_offset,
        )
    return SourcePosition(
        line=pos.line + base_line,
        character=pos.character,
        offset=pos.offset + base_offset,
    )


def _shift_range(rng: Range, base_line: int, base_char: int, base_offset: int) -> Range:
    """Shift both endpoints of *rng*."""
    return Range(
        start=_shift_position(rng.start, base_line, base_char, base_offset),
        end=_shift_position(rng.end, base_line, base_char, base_offset),
    )


def _shift_scope(scope: Scope, base_line: int, base_char: int, base_offset: int) -> Scope:
    """Recursively shift all ranges inside a scope tree."""
    new_body_range = (
        _shift_range(scope.body_range, base_line, base_char, base_offset)
        if scope.body_range is not None
        else None
    )
    new_vars: dict[str, VarDef] = {}
    for vname, vdef in scope.variables.items():
        new_refs = [_shift_range(r, base_line, base_char, base_offset) for r in vdef.references]
        new_vars[vname] = VarDef(
            name=vdef.name,
            definition_range=_shift_range(vdef.definition_range, base_line, base_char, base_offset),
            references=new_refs,
            warn_if_unused=vdef.warn_if_unused,
        )
    new_procs: dict[str, ProcDef] = {}
    for pname, pdef in scope.procs.items():
        new_procs[pname] = _shift_proc(pdef, base_line, base_char, base_offset)
    new_classes: dict[str, ClassDef] = {}
    for cname, cdef in scope.classes.items():
        new_classes[cname] = _shift_class(cdef, base_line, base_char, base_offset)
    new_scope = Scope(
        kind=scope.kind,
        name=scope.name,
        parent=scope.parent,
        body_range=new_body_range,
        variables=new_vars,
        procs=new_procs,
        classes=new_classes,
    )
    new_scope.children = [
        _shift_scope(child, base_line, base_char, base_offset) for child in scope.children
    ]
    for child in new_scope.children:
        child.parent = new_scope
    return new_scope


def _shift_proc(pdef: ProcDef, base_line: int, base_char: int, base_offset: int) -> ProcDef:
    """Shift all ranges in a ProcDef."""
    return ProcDef(
        name=pdef.name,
        qualified_name=pdef.qualified_name,
        params=list(pdef.params),
        name_range=_shift_range(pdef.name_range, base_line, base_char, base_offset),
        body_range=_shift_range(pdef.body_range, base_line, base_char, base_offset),
        doc=pdef.doc,
        param_traits=dict(pdef.param_traits),
    )


def _shift_method(mdef: MethodDef, base_line: int, base_char: int, base_offset: int) -> MethodDef:
    """Shift all ranges in a MethodDef."""
    return MethodDef(
        name=mdef.name,
        params=mdef.params,
        name_range=_shift_range(mdef.name_range, base_line, base_char, base_offset),
        body_range=_shift_range(mdef.body_range, base_line, base_char, base_offset),
        visibility=mdef.visibility,
        kind=mdef.kind,
        doc=mdef.doc,
        param_traits=dict(mdef.param_traits),
    )


def _shift_property(
    pdef: PropertyDef, base_line: int, base_char: int, base_offset: int
) -> PropertyDef:
    """Shift all ranges in a PropertyDef."""
    return PropertyDef(
        name=pdef.name,
        name_range=_shift_range(pdef.name_range, base_line, base_char, base_offset),
        kind=pdef.kind,
        has_getter=pdef.has_getter,
        has_setter=pdef.has_setter,
    )


def _shift_class(cdef: ClassDef, base_line: int, base_char: int, base_offset: int) -> ClassDef:
    """Shift all ranges in a ClassDef."""
    new_methods = {
        n: _shift_method(m, base_line, base_char, base_offset) for n, m in cdef.methods.items()
    }
    new_class_methods = {
        n: _shift_method(m, base_line, base_char, base_offset)
        for n, m in cdef.class_methods.items()
    }
    new_constructors = [
        _shift_method(m, base_line, base_char, base_offset) for m in cdef.constructors
    ]
    new_destructor = (
        _shift_method(cdef.destructor, base_line, base_char, base_offset)
        if cdef.destructor
        else None
    )
    new_properties = {
        n: _shift_property(p, base_line, base_char, base_offset) for n, p in cdef.properties.items()
    }
    return ClassDef(
        name=cdef.name,
        qualified_name=cdef.qualified_name,
        name_range=_shift_range(cdef.name_range, base_line, base_char, base_offset),
        body_range=_shift_range(cdef.body_range, base_line, base_char, base_offset),
        metaclass=cdef.metaclass,
        superclasses=list(cdef.superclasses),
        mixins=list(cdef.mixins),
        methods=new_methods,
        class_methods=new_class_methods,
        constructors=new_constructors,
        destructor=new_destructor,
        variables=list(cdef.variables),
        properties=new_properties,
        filters=list(cdef.filters),
        exports=set(cdef.exports),
        unexports=set(cdef.unexports),
        doc=cdef.doc,
    )


def _shift_diagnostic(
    diag: Diagnostic, base_line: int, base_char: int, base_offset: int
) -> Diagnostic:
    """Shift all ranges in a Diagnostic (including fixes and related_ranges)."""
    new_fixes = tuple(
        CodeFix(
            range=_shift_range(fix.range, base_line, base_char, base_offset),
            new_text=fix.new_text,
            description=fix.description,
        )
        for fix in diag.fixes
    )
    new_related = tuple(
        (_shift_range(r, base_line, base_char, base_offset), msg) for r, msg in diag.related_ranges
    )
    return Diagnostic(
        range=_shift_range(diag.range, base_line, base_char, base_offset),
        message=diag.message,
        severity=diag.severity,
        code=diag.code,
        fixes=new_fixes,
        related_ranges=new_related,
    )


def _merge_shifted_result(
    merged: AnalysisResult,
    result: AnalysisResult,
    base_line: int,
    base_char: int,
    base_offset: int,
) -> None:
    """Shift all ranges in *result* and merge into *merged*."""
    # Diagnostics
    for diag in result.diagnostics:
        merged.diagnostics.append(_shift_diagnostic(diag, base_line, base_char, base_offset))

    # Suppressed lines — negative keys are sentinels (e.g. _FILE_SUPPRESS_KEY)
    # that must not be shifted; only real line numbers (>= 0) need an offset.
    for line_no, codes in result.suppressed_lines.items():
        merged.suppressed_lines[line_no if line_no < 0 else line_no + base_line] = codes

    # Regex patterns
    for rp in result.regex_patterns:
        merged.regex_patterns.append(
            RegexPattern(
                range=_shift_range(rp.range, base_line, base_char, base_offset),
                pattern=rp.pattern,
                command=rp.command,
            )
        )

    # Command invocations
    for ci in result.command_invocations:
        merged.command_invocations.append(
            CommandInvocation(
                name=ci.name,
                range=_shift_range(ci.range, base_line, base_char, base_offset),
                resolved_qualified_name=ci.resolved_qualified_name,
            )
        )

    # Package requires/provides
    for pr in result.package_requires:
        merged.package_requires.append(
            PackageRequire(
                name=pr.name,
                version=pr.version,
                range=_shift_range(pr.range, base_line, base_char, base_offset),
                conditional=pr.conditional,
            )
        )
    for pp in result.package_provides:
        merged.package_provides.append(
            PackageProvide(
                name=pp.name,
                version=pp.version,
                range=_shift_range(pp.range, base_line, base_char, base_offset),
            )
        )

    # Source targets
    for st in result.source_targets:
        merged.source_targets.append(
            SourceTarget(
                raw_path=st.raw_path,
                range=_shift_range(st.range, base_line, base_char, base_offset),
                is_literal=st.is_literal,
            )
        )

    # Stub commands and expr defs
    for stub in result.stub_commands:
        merged.stub_commands.append(
            StubCommandDef(
                name=stub.name,
                args=stub.args,
                range=_shift_range(stub.range, base_line, base_char, base_offset),
                barrier=stub.barrier,
                loop=stub.loop,
                pure=stub.pure,
                mutator=stub.mutator,
                unsafe=stub.unsafe,
                scope_alias=stub.scope_alias,
            )
        )
    for stub_expr in result.stub_expr_defs:
        merged.stub_expr_defs.append(
            StubExprDef(
                name=stub_expr.name,
                kind=stub_expr.kind,
                arity=stub_expr.arity,
                pure=stub_expr.pure,
                range=_shift_range(stub_expr.range, base_line, base_char, base_offset),
            )
        )

    # Procs and classes (merged into global scope)
    for pname, pdef in result.all_procs.items():
        merged.all_procs[pname] = _shift_proc(pdef, base_line, base_char, base_offset)
    for cname, cdef in result.all_classes.items():
        merged.all_classes[cname] = _shift_class(cdef, base_line, base_char, base_offset)
    # Variables
    for vname, vdef in result.all_variables.items():
        new_refs = [_shift_range(r, base_line, base_char, base_offset) for r in vdef.references]
        merged.all_variables[vname] = VarDef(
            name=vdef.name,
            definition_range=_shift_range(vdef.definition_range, base_line, base_char, base_offset),
            references=new_refs,
            warn_if_unused=vdef.warn_if_unused,
        )

    # Command aliases
    for alias_name, alias_val in result.command_aliases.items():
        merged.command_aliases[alias_name] = alias_val

    # Dynamic providers
    if result.has_dynamic_providers:
        merged.has_dynamic_providers = True

    # Unknown proc info
    if result.unknown_proc_info is not None and merged.unknown_proc_info is None:
        merged.unknown_proc_info = result.unknown_proc_info

    # Scope tree: add the shifted per-body global scope as a child of
    # the merged global scope so features that walk the scope tree
    # (e.g. document symbols, variable lookup) find the right context.
    shifted_scope = _shift_scope(result.global_scope, base_line, base_char, base_offset)
    shifted_scope.parent = merged.global_scope
    merged.global_scope.children.append(shifted_scope)


def _diag_command_name(diag: Diagnostic) -> str | None:
    """Extract the command name from a W123 diagnostic message.

    W123 messages follow the pattern ``Unknown command 'name'``.
    """
    msg = diag.message
    start = msg.find("'")
    end = msg.find("'", start + 1) if start >= 0 else -1
    if start >= 0 and end > start:
        return msg[start + 1 : end]
    return None


def analyse_conf_wrapped(
    source: str,
    *,
    disabled_diagnostics: frozenset[str] | None = None,
    file_path: str | None = None,
) -> tuple[AnalysisResult, list[EmbeddedRule]]:
    """Analyse all rule bodies in a conf-wrapped iRules file.

    Returns ``(merged_result, embedded_rules)`` where *merged_result*
    contains diagnostics, scopes, procs, and symbols from all rule
    bodies with ranges shifted to file coordinates.  All rules share
    procs and RULE_INIT variables via flat sharing.
    """
    embedded_rules = find_embedded_rules(source)
    if not embedded_rules:
        return AnalysisResult(), []

    buf = DocumentBuffer.from_source(source)

    # Phase 1: Analyse each rule body independently.
    per_rule: list[tuple[EmbeddedRule, AnalysisResult]] = []
    for rule in embedded_rules:
        analyser = Analyser(disabled_diagnostics=disabled_diagnostics)
        result = analyser.analyse(rule.body, file_path=file_path)
        per_rule.append((rule, result))

    # Phase 2: Cross-rule proc sharing — collect all proc names from
    # all rules, then suppress false W123 (unresolved command)
    # diagnostics that reference procs defined in other rules.
    all_proc_names: set[str] = set()
    for _rule, result in per_rule:
        for qname in result.all_procs:
            all_proc_names.add(qname)
            tail = qname.rsplit("::", 1)[-1]
            if tail:
                all_proc_names.add(tail)

    if all_proc_names:
        for _rule, result in per_rule:
            local_procs = set(result.all_procs.keys())
            local_tails = {qn.rsplit("::", 1)[-1] for qn in local_procs if "::" in qn}
            cross_procs = all_proc_names - local_procs - local_tails
            if cross_procs:
                result.diagnostics = [
                    d
                    for d in result.diagnostics
                    if not (d.code == "W123" and _diag_command_name(d) in cross_procs)
                ]

    # Phase 3: Merge results with shifted ranges.
    merged = AnalysisResult()
    for rule, result in per_rule:
        body_pos = buf.offset_to_position(rule.body_start_offset)
        base_line = body_pos.line
        base_char = body_pos.character
        base_offset = rule.body_start_offset
        _merge_shifted_result(merged, result, base_line, base_char, base_offset)

    return merged, embedded_rules
