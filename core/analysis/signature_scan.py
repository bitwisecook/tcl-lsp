"""Signature-only scan for background-indexed Tcl files.

``extract_signatures(source)`` returns an :class:`AnalysisResult` populated
with just the fields cross-file LSP features read for non-OPEN documents:
``all_procs``, ``all_classes``, ``package_requires``, ``source_targets``,
and ``command_aliases``. Diagnostics, optimiser passes, variable-reference
tracking, lowering, and every other heavy analyser stage are skipped.

The full ``analyse()`` pipeline runs on ``didOpen`` / ``didChange`` so
currently-open documents are unaffected. Background-scanned files —
workspace discovery, ``package require`` resolution, tclIndex entries —
call into this module instead, which is both orders of magnitude faster
and retains orders of magnitude less memory per file than ``analyse()``.
"""

from __future__ import annotations

from core.analysis.analyser import _parse_param_list
from core.analysis.semantic_model import (
    AnalysisResult,
    ClassDef,
    PackageRequire,
    ProcDef,
    SourceTarget,
)
from core.common.ranges import range_from_token
from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import Token


def extract_signatures(source: str) -> AnalysisResult:
    """Return a minimal AnalysisResult for a background-indexed file."""
    result = AnalysisResult()
    _scan(source, body_token=None, ns_prefix="", result=result)
    return result


def _qualify(ns_prefix: str, name: str) -> str:
    """Fully qualify *name* within *ns_prefix* following Tcl scoping."""
    if name.startswith("::"):
        return name
    if ns_prefix:
        return f"::{ns_prefix}::{name}"
    return f"::{name}"


def _scan(
    source: str,
    body_token: Token | None,
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    commands = segment_commands(source, body_token=body_token, recovery=False)
    for cmd in commands:
        if cmd.is_partial or not cmd.argv:
            continue
        texts = cmd.texts
        argv = cmd.argv
        head = texts[0] if texts else ""
        match head:
            case "proc":
                _handle_proc(texts, argv, ns_prefix, result)
            case "namespace":
                _handle_namespace(texts, argv, ns_prefix, result)
            case "package":
                _handle_package(texts, argv, result)
            case "source":
                _handle_source(texts, argv, result)
            case "interp":
                _handle_interp(texts, result)
            case "oo::class":
                _handle_oo_class(texts, argv, ns_prefix, result)
            case "itcl::class" | "::itcl::class":
                _handle_itcl_class(texts, argv, ns_prefix, result)


def _handle_proc(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    if len(texts) < 4:
        return
    raw_name = texts[1]
    qualified = _qualify(ns_prefix, raw_name)
    simple = qualified.rsplit("::", 1)[-1]
    name_range = range_from_token(argv[1])
    body_range = range_from_token(argv[3]) if len(argv) > 3 else name_range
    result.all_procs[qualified] = ProcDef(
        name=simple,
        qualified_name=qualified,
        params=_parse_param_list(texts[2]),
        name_range=name_range,
        body_range=body_range,
    )


def _handle_namespace(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    if len(texts) < 4 or texts[1] != "eval":
        return
    ns_name = texts[2].lstrip(":")
    inner_prefix = ns_name if not ns_prefix else f"{ns_prefix}::{ns_name}"
    _scan(texts[3], body_token=argv[3], ns_prefix=inner_prefix, result=result)


def _handle_package(
    texts: list[str],
    argv: list[Token],
    result: AnalysisResult,
) -> None:
    if len(texts) < 3 or texts[1] != "require":
        return
    # ``package require ?-exact? NAME ?VERSION ...?``
    idx = 2
    if texts[idx] == "-exact" and len(texts) > idx + 1:
        idx += 1
    pkg_name = texts[idx]
    version = texts[idx + 1] if len(texts) > idx + 1 else None
    result.package_requires.append(
        PackageRequire(
            name=pkg_name,
            version=version,
            range=range_from_token(argv[idx]),
        )
    )


def _handle_source(
    texts: list[str],
    argv: list[Token],
    result: AnalysisResult,
) -> None:
    # ``source ?-encoding ENC? PATH`` — any further options are ignored
    idx = 1
    while idx < len(texts) and texts[idx].startswith("-"):
        if texts[idx] == "-encoding" and idx + 1 < len(texts):
            idx += 2
        else:
            idx += 1
    if idx >= len(texts):
        return
    # ``texts[idx]`` is the reconstructed word with variable and command
    # substitutions re-wrapped as ``${var}`` / ``[cmd]``, so those markers
    # are reliable evidence that the path is not a plain literal.
    raw = texts[idx]
    is_literal = "$" not in raw and "[" not in raw
    result.source_targets.append(
        SourceTarget(
            raw_path=raw,
            range=range_from_token(argv[idx]),
            is_literal=is_literal,
        )
    )


def _handle_interp(texts: list[str], result: AnalysisResult) -> None:
    # ``interp alias {} NAME {} TARGET ?ARG ...?``
    if len(texts) < 6 or texts[1] != "alias":
        return
    alias_name = texts[3]
    target = texts[5]
    extras = tuple(texts[6:])
    result.command_aliases[_qualify("", alias_name)] = (target, extras)


def _handle_oo_class(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    # ``oo::class create NAME ?BODY?``
    if len(texts) < 3 or texts[1] != "create":
        return
    _emit_class(texts[2], argv[2], argv[3] if len(argv) > 3 else argv[2], ns_prefix, result)


def _handle_itcl_class(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    # ``itcl::class NAME BODY``
    if len(texts) < 3:
        return
    _emit_class(texts[1], argv[1], argv[2], ns_prefix, result)


def _emit_class(
    raw_name: str,
    name_tok: Token,
    body_tok: Token,
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    qualified = _qualify(ns_prefix, raw_name.lstrip(":"))
    simple = qualified.rsplit("::", 1)[-1]
    result.all_classes[qualified] = ClassDef(
        name=simple,
        qualified_name=qualified,
        name_range=range_from_token(name_tok),
        body_range=range_from_token(body_tok),
    )
