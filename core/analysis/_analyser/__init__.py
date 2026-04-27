from __future__ import annotations

import logging

from ..semantic_model import (
    AnalysisResult,
    AutoPathEntry,
    ClassDef,
    CodeFix,
    CommandInvocation,
    Diagnostic,
    MethodDef,
    NamespaceImport,
    PackageProvide,
    PackageRequire,
    ParamDef,
    ProcDef,
    PropertyDef,
    RegexPattern,
    Scope,
    Severity,
    SourceTarget,
    StubCommandDef,
    StubExprDef,
    UnknownProcInfo,
    VarDef,
)
from ._commands import _AnalyserCommandsMixin
from ._core import _AnalyserBase
from ._diag_branches import _AnalyserDiagBranchesMixin
from ._diag_channel import _AnalyserDiagChannelMixin
from ._diag_commands import _AnalyserDiagCommandsMixin
from ._diag_ip import _AnalyserDiagIPMixin
from ._diag_racy import _AnalyserDiagRacyMixin
from ._diag_var_command import _AnalyserDiagVarCommandMixin
from ._diag_var_lifecycle import _AnalyserDiagVarLifecycleMixin
from ._diagnostics import _AnalyserDiagsMixin
from ._handlers import _AnalyserHandlersMixin
from ._oo import _AnalyserOOMixin
from ._proc import _AnalyserProcMixin
from ._recovery import _AnalyserRecoveryMixin
from ._scope import _AnalyserScopeMixin
from ._snapshot import AnalyserSnapshot
from ._utils import parse_param_list

log = logging.getLogger(__name__)

# Optional Rust binding — imported best-effort so the Python path
# still loads cleanly when the wheel isn't installed.
try:  # pragma: no cover - import-time fallback
    from tcl_lsp_rust import analyser_analyse as _rust_analyse  # type: ignore[import-not-found]
except ImportError:  # pragma: no cover
    _rust_analyse = None  # type: ignore[assignment]


class Analyser(
    _AnalyserRecoveryMixin,
    _AnalyserScopeMixin,
    _AnalyserCommandsMixin,
    _AnalyserHandlersMixin,
    _AnalyserProcMixin,
    _AnalyserOOMixin,
    _AnalyserDiagsMixin,
    _AnalyserDiagCommandsMixin,
    _AnalyserDiagVarCommandMixin,
    _AnalyserDiagBranchesMixin,
    _AnalyserDiagChannelMixin,
    _AnalyserDiagIPMixin,
    _AnalyserDiagRacyMixin,
    _AnalyserDiagVarLifecycleMixin,
    _AnalyserBase,  # last — must follow all mixins that inherit from it (TYPE_CHECKING)
):
    "Single-pass Tcl analyser assembled from mixin groups."


_SEVERITY_MAP: dict[str, Severity] = {
    "hint": Severity.HINT,
    "suggestion": Severity.HINT,  # Python has no Suggestion variant; map to Hint.
    "warning": Severity.WARNING,
    "error": Severity.ERROR,
}


def _materialise_rust_analysis(source: str, raw: dict) -> AnalysisResult:
    """Convert the dict returned by ``tcl_lsp_rust.analyser_analyse``
    into an :class:`AnalysisResult`.

    Spans on the Rust side are ``(start, end)`` ``u32`` tuples; we
    resolve them to LSP :class:`Range` values via
    :func:`core.compiler.rust_spans.build_position_resolver`.

    The Rust analyser is partial — fields it doesn't populate
    (``stub_commands``, ``unknown_proc_info``, ``auto_path_entries``,
    ``regex_patterns``, …) default to empty.  Differential testing
    in C41f5 surfaces parity gaps over time; failures cleanly
    fall back to the Python path via the ``analyse`` shim.
    """
    from core.compiler.rust_spans import build_position_resolver

    _, range_at = build_position_resolver(source)
    result = AnalysisResult()

    def _params(params_raw: list[dict]) -> list[ParamDef]:
        return [
            ParamDef(
                name=p["name"],
                has_default=p["has_default"],
                default_value=p.get("default_value") or "",
            )
            for p in params_raw
        ]

    def _proc(p: dict) -> ProcDef:
        return ProcDef(
            name=p["name"],
            qualified_name=p["qualified_name"],
            params=_params(p["params"]),
            name_range=range_at(*p["name_range"]),
            body_range=range_at(*p["body_range"]),
            doc=p.get("doc") or "",
        )

    def _method(m: dict) -> MethodDef:
        return MethodDef(
            name=m["name"],
            params=tuple(_params(m.get("params", []))),
            name_range=range_at(*m["name_range"]),
            body_range=range_at(*m["body_range"]),
            visibility=m.get("visibility") or "public",
            kind=m.get("kind") or "method",
            doc=m.get("doc") or "",
        )

    def _property(p: dict) -> PropertyDef:
        return PropertyDef(
            name=p["name"],
            name_range=range_at(*p["name_range"]),
            kind=p.get("kind") or "readwrite",
            has_getter=p.get("has_getter", False),
            has_setter=p.get("has_setter", False),
        )

    def _class(c: dict) -> ClassDef:
        cd = ClassDef(
            name=c["name"],
            qualified_name=c["qualified_name"],
            name_range=range_at(*c["name_range"]),
            body_range=range_at(*c["body_range"]),
        )
        # The Rust analyser populates these structural fields from
        # C41e0+e1+e2 onwards.  Older binding builds may omit them
        # — fall back to the dataclass defaults.
        if "metaclass" in c:
            cd.metaclass = c["metaclass"]
        if "superclasses" in c:
            cd.superclasses = list(c["superclasses"])
        if "mixins" in c:
            cd.mixins = list(c["mixins"])
        for name, m in c.get("methods", {}).items():
            cd.methods[name] = _method(m)
        for name, m in c.get("class_methods", {}).items():
            cd.class_methods[name] = _method(m)
        # C41e3 fields — present when the Rust binding is C41e3+;
        # older builds omit them and we keep the dataclass defaults.
        for ctor in c.get("constructors", []):
            cd.constructors.append(_method(ctor))
        if c.get("destructor") is not None:
            cd.destructor = _method(c["destructor"])
        if "variables" in c:
            cd.variables = list(c["variables"])
        for name, p in c.get("properties", {}).items():
            cd.properties[name] = _property(p)
        if "filters" in c:
            cd.filters = list(c["filters"])
        if "exports" in c:
            cd.exports = set(c["exports"])
        if "unexports" in c:
            cd.unexports = set(c["unexports"])
        if "doc" in c:
            cd.doc = c["doc"]
        return cd

    def _var(v: dict) -> VarDef:
        return VarDef(
            name=v["name"],
            definition_range=range_at(*v["definition_range"]),
            references=[range_at(*r) for r in v.get("references", [])],
            warn_if_unused=v["warn_if_unused"],
        )

    def _scope(s: dict) -> Scope:
        scope = Scope(
            kind=s["kind"],
            name=s["name"],
            body_range=range_at(*s["body_range"]) if s.get("body_range") else None,
        )
        for name, v in s.get("variables", {}).items():
            scope.variables[name] = _var(v)
        for name, p in s.get("procs", {}).items():
            scope.procs[name] = _proc(p)
        for name, c in s.get("classes", {}).items():
            scope.classes[name] = _class(c)
        for child in s.get("children", []):
            scope.children.append(_scope(child))
        return scope

    result.global_scope = _scope(raw["global_scope"])
    for qname, p in raw.get("all_procs", {}).items():
        result.all_procs[qname] = _proc(p)
    for qname, c in raw.get("all_classes", {}).items():
        result.all_classes[qname] = _class(c)
    for qname, v in raw.get("all_variables", {}).items():
        result.all_variables[qname] = _var(v)
    for d in raw.get("diagnostics", []):
        fixes_raw = d.get("fixes") or ()
        fixes = tuple(
            CodeFix(
                range=range_at(*f["range"]),
                new_text=f.get("new_text") or "",
                description=f.get("description") or "",
            )
            for f in fixes_raw
        )
        result.diagnostics.append(
            Diagnostic(
                range=range_at(*d["range"]),
                message=d["message"],
                severity=_SEVERITY_MAP.get(d["severity"], Severity.WARNING),
                code=d["code"],
                fixes=fixes,
            )
        )

    # Side-channel fields the Rust dict already populates.
    # Workspace-index, alias-driven role propagation, and
    # ``package require``-keyed feature gating all rely on these
    # being threaded through to the materialised result.
    #
    # ``resolved_qualified_name`` is **not** populated by the Rust
    # binding yet; resolve it Python-side here against the
    # materialised ``all_procs`` so workspace usage counts (``::name``
    # → invocation count) keep working under default-on.
    proc_name_set = frozenset(result.all_procs.keys())
    for inv in raw.get("command_invocations", []):
        cmd_name = inv["name"]
        resolved = inv.get("resolved_qualified_name")
        if resolved is None and cmd_name and proc_name_set:
            from ...common.naming import normalise_qualified_name as _nq

            if cmd_name.startswith("::"):
                candidate = _nq(cmd_name)
            elif "::" in cmd_name:
                candidate = _nq(f"::{cmd_name}")
            else:
                candidate = _nq(f"::{cmd_name}")
            if candidate in proc_name_set:
                resolved = candidate
        result.command_invocations.append(
            CommandInvocation(
                name=cmd_name,
                range=range_at(*inv["range"]),
                resolved_qualified_name=resolved,
            )
        )
    for pr in raw.get("package_requires", []):
        result.package_requires.append(
            PackageRequire(
                name=pr["name"],
                version=pr.get("version"),
                range=range_at(*pr["range"]),
                conditional=pr.get("conditional", False),
            )
        )
    for st in raw.get("source_targets", []):
        result.source_targets.append(
            SourceTarget(
                raw_path=st["raw_path"],
                range=range_at(*st["range"]),
                is_literal=st.get("is_literal", True),
            )
        )
    for ni in raw.get("namespace_imports", []):
        result.namespace_imports.append(
            NamespaceImport(
                ns=ni["ns"],
                pattern=ni["pattern"],
                range=range_at(*ni["range"]),
                conjectured=ni.get("conjectured", False),
            )
        )
    for qname, alias in (raw.get("command_aliases") or {}).items():
        result.command_aliases[qname] = (
            alias["target"],
            tuple(alias.get("extras") or ()),
        )
    for pp in raw.get("package_provides", []):
        result.package_provides.append(
            PackageProvide(
                name=pp["name"],
                version=pp.get("version"),
                range=range_at(*pp["range"]),
            )
        )
    if raw.get("has_dynamic_providers"):
        result.has_dynamic_providers = True
    for ap in raw.get("auto_path_entries", []):
        result.auto_path_entries.append(
            AutoPathEntry(
                resolved_path=None,
                raw=ap["raw_path"],
                range=range_at(*ap["range"]),
            )
        )
    # The Rust stub scanner only carries names + ranges today;
    # arg-roles / flags (``-loop`` / ``-barrier`` / ``-pure`` …)
    # remain on the Python supplement until a richer port lands.
    for sc in raw.get("stub_commands", []):
        result.stub_commands.append(
            StubCommandDef(
                name=sc["name"],
                args=(),
                range=range_at(*sc["range"]),
            )
        )
    for se in raw.get("stub_expr_defs", []):
        result.stub_expr_defs.append(
            StubExprDef(
                name=se["name"],
                kind="function",
                range=range_at(*se["range"]),
            )
        )
    for rp in raw.get("regex_patterns", []):
        result.regex_patterns.append(
            RegexPattern(
                range=range_at(*rp["range"]),
                pattern=rp["pattern"],
                command=rp["command"],
            )
        )
    for line, codes in (raw.get("suppressed_lines") or {}).items():
        result.suppressed_lines[int(line)] = frozenset(codes)

    # C41e3 — UnknownProcInfo from a user-defined ``unknown`` proc.
    upi = raw.get("unknown_proc_info")
    if upi is not None:
        result.unknown_proc_info = UnknownProcInfo(
            dispatch_targets=frozenset(upi.get("dispatch_targets", [])),
            chains_original=upi.get("chains_original", False),
            empty_stub=upi.get("empty_stub", False),
            case_insensitive=upi.get("case_insensitive", False),
            has_pattern_dispatch=upi.get("has_pattern_dispatch", False),
            has_exec=upi.get("has_exec", False),
            has_auto_load=upi.get("has_auto_load", False),
        )
    return result


def _merge_rust_with_python_supplement(
    rust: AnalysisResult, python: AnalysisResult
) -> AnalysisResult:
    """Merge Rust structural data with Python-supplied side channels.

    The Rust analyser populates the structural payload (``all_procs``,
    ``all_classes``, ``all_variables``, ``global_scope``,
    ``unknown_proc_info``, plus the ``command_invocations`` /
    ``command_aliases`` / ``package_requires`` side channels). It does
    **not** yet emit ``source_targets``, ``namespace_imports``,
    ``regex_patterns``, ``stub_commands``, ``stub_expr_defs``,
    ``auto_path_entries``, ``package_provides``, ``suppressed_lines``,
    or the post-pass diagnostics that Python's ``Analyser`` integrates
    via ``run_compiler_checks`` (W110 / W220 / W304, …).

    For the default-on transition we keep Rust as the authoritative
    source for the structural fields and copy the Python pass's
    side-channel data over. As the Rust port lands the missing
    emitters those copies become no-ops (Rust will populate them
    first); the merger then becomes a thin shim that future chunks
    can delete.
    """
    # Rust ports many of these fields now (see
    # ``_materialise_rust_analysis``) but Python's pass remains the
    # source of truth for several scenarios where Rust's scanners
    # are still partial:
    #
    # - ``regex_patterns``: variable-driven pattern propagation
    #   (``set p "..."; switch -regexp -- $p { ... }``) and
    #   deep-recursion patterns aren't ported.
    # - ``stub_commands`` / ``stub_expr_defs``: arg-roles, flags
    #   (``-loop`` / ``-barrier`` / ``-pure``), and the per-arg
    #   ``StubArgDef`` machinery are Python-only.
    # - ``auto_path_entries``: ``[info script]`` resolution and
    #   substitution evaluation are Python-only.
    # - ``namespace_imports`` / ``source_targets``: tcllib-style
    #   wrapper-call detection (``X::import`` / dynamic source
    #   targets) is Python-only.
    # - ``command_aliases``: ``interp alias`` chains across
    #   namespace boundaries are Python-only.
    # - ``command_invocations``: invocations inside method /
    #   when bodies are Python-walked deeper than the Rust
    #   ``ArgRole::Body`` loop reaches today.
    #
    # Each "Python over Rust" line is a future Rust-port
    # follow-up; trimming this block is the way to slim the
    # supplement.
    rust.source_targets = python.source_targets
    rust.regex_patterns = python.regex_patterns
    rust.stub_commands = python.stub_commands
    rust.stub_expr_defs = python.stub_expr_defs
    rust.auto_path_entries = python.auto_path_entries
    rust.package_provides = python.package_provides
    rust.has_dynamic_providers = python.has_dynamic_providers
    rust.suppressed_lines = python.suppressed_lines
    if not rust.namespace_imports:
        rust.namespace_imports = python.namespace_imports
    if not rust.command_aliases:
        rust.command_aliases = python.command_aliases
    rust.command_invocations = python.command_invocations
    # Python's diagnostics list is a strict superset (it integrates
    # ``run_compiler_checks`` for W110 / W220 / W304).  Take Python's.
    rust.diagnostics = python.diagnostics
    # Structural Rust gaps the differential corpus tracks:
    # ``ProcDef.doc`` (preceding-comment extraction works for
    # most cases now but extract_body_docstring fallback is
    # Python-only); ``ProcDef.param_traits``;
    # ``unknown_proc_info`` patched-``lower_to_ir`` test path.
    # Take Python's structural data wholesale until each gap
    # closes.
    rust.global_scope = python.global_scope
    rust.all_variables = python.all_variables
    rust.all_procs = python.all_procs
    rust.all_classes = python.all_classes
    rust.unknown_proc_info = python.unknown_proc_info
    return rust


def analyse(source: str, cu=None) -> AnalysisResult:
    """Analyse `source` for the active dialect.

    Dispatches to the Rust port when ``TCL_LSP_RUST_ANALYSER`` is
    set to a truthy value (or unset, after the C41-default-on flip)
    AND no caller-provided :class:`CompilationUnit` is supplied
    (the Rust path builds its own CU internally; reusing a
    pre-built one isn't supported yet).  Default polarity is
    **OFF** — set ``TCL_LSP_RUST_ANALYSER=1`` to opt in.

    Under the env var the Rust binding is consulted for the
    structural payload; a Python supplementary pass fills in the
    side-channel fields the Rust port doesn't emit yet
    (see :func:`_merge_rust_with_python_supplement`).  Any
    exception raised by the Rust path is logged at DEBUG and the
    Python path runs alone as a safety net.
    """
    from core.common.dialect import active_dialect
    from core.compiler.rust_spans import rust_shim_enabled

    if (
        cu is None
        and _rust_analyse is not None
        and rust_shim_enabled("TCL_LSP_RUST_ANALYSER", default=True)
    ):
        try:
            rust_result = _materialise_rust_analysis(
                source, _rust_analyse(source, active_dialect())
            )
            python_result = Analyser().analyse(source, cu=cu)
            return _merge_rust_with_python_supplement(rust_result, python_result)
        except Exception:  # pragma: no cover - safety net
            log.debug("rust analyser failed, falling back to python", exc_info=True)
    return Analyser().analyse(source, cu=cu)


__all__ = ["Analyser", "AnalyserSnapshot", "AnalysisResult", "parse_param_list", "analyse"]
