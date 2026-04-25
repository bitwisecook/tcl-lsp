"""Signature-only scan for background-indexed Tcl files.

``extract_signatures(source)`` returns an :class:`AnalysisResult` populated
with just the fields cross-file LSP features read for non-OPEN documents:
``all_procs``, ``all_classes``, ``package_requires``, ``source_targets``,
``command_aliases``, and a lightweight ``command_invocations`` list
carrying ``name + range`` for every top-level and branch-body command so
``WorkspaceIndex.command_usage_counts()`` still reflects background files.

Diagnostics, regex patterns, stub definitions, suppressed-line sets,
variable-reference tracking, the scope tree, and every other heavy
analyser stage are skipped; ``CommandInvocation.resolved_qualified_name``
is left ``None`` because resolving requires the full scope walk, so
``proc_usage_counts()`` under-approximates for cross-file references
into background files (it is an approximation aid, not a correctness
signal).

The full ``analyse()`` pipeline still runs on ``didOpen`` / ``didChange``
so currently-open documents are unaffected. Background-scanned files —
workspace discovery, ``package require`` resolution, tclIndex entries —
call into this module instead, which is both orders of magnitude faster
and retains orders of magnitude less memory per file than ``analyse()``.
"""

from __future__ import annotations

import logging
import os
from dataclasses import dataclass, field

from core.analysis import parse_param_list
from core.analysis.semantic_model import (
    AnalysisResult,
    AutoPathEntry,
    ClassDef,
    CommandInvocation,
    NamespaceImport,
    PackageRequire,
    ParamDef,
    ProcDef,
    SourceTarget,
)
from core.common.ranges import range_from_token
from core.compiler.rust_spans import build_position_resolver
from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import Token, TokenType

log = logging.getLogger(__name__)

try:
    from tcl_lsp_rust import (  # ty: ignore[unresolved-import]
        signature_scan_extract as _rust_extract,
    )
except ImportError:  # pragma: no cover - rust binding is optional
    _rust_extract = None


def _materialise_rust_signatures(source: str, raw: dict) -> AnalysisResult:
    """Convert the dict returned by ``tcl_lsp_rust.signature_scan_extract``
    into an :class:`AnalysisResult`.

    Spans on the Rust side are ``(start, end)`` ``u32`` tuples; we
    resolve them to LSP :class:`Range` values via
    :func:`core.compiler.rust_spans.build_position_resolver`.
    """
    _, range_at = build_position_resolver(source)
    result = AnalysisResult()

    def _params(params_raw: list[dict]) -> list[ParamDef]:
        return [
            ParamDef(
                name=p["name"],
                has_default=p["has_default"],
                default_value=p["default_value"] or "",
            )
            for p in params_raw
        ]

    for qname, p in raw.get("procs", {}).items():
        name_range = range_at(*p["name_range"])
        body_range = range_at(*p["body_range"])
        result.all_procs[qname] = ProcDef(
            name=p["name"],
            qualified_name=p["qualified_name"],
            params=_params(p["params"]),
            name_range=name_range,
            body_range=body_range,
        )

    for qname, c in raw.get("classes", {}).items():
        result.all_classes[qname] = ClassDef(
            name=c["name"],
            qualified_name=c["qualified_name"],
            name_range=range_at(*c["name_range"]),
            body_range=range_at(*c["body_range"]),
        )

    for inv in raw.get("command_invocations", []):
        result.command_invocations.append(
            CommandInvocation(name=inv["name"], range=range_at(*inv["range"]))
        )

    for pr in raw.get("package_requires", []):
        result.package_requires.append(
            PackageRequire(
                name=pr["name"],
                version=pr["version"],
                range=range_at(*pr["range"]),
                conditional=pr["conditional"],
            )
        )

    for s in raw.get("source_targets", []):
        result.source_targets.append(
            SourceTarget(
                raw_path=s["raw_path"],
                range=range_at(*s["range"]),
                is_literal=s["is_literal"],
            )
        )

    for qname, a in raw.get("command_aliases", {}).items():
        result.command_aliases[qname] = (a["target"], tuple(a["extras"]))

    for imp in raw.get("namespace_imports", []):
        result.namespace_imports.append(
            NamespaceImport(
                ns=imp["ns"],
                pattern=imp["pattern"],
                range=range_at(*imp["range"]),
                conjectured=imp["conjectured"],
            )
        )

    for entry in raw.get("auto_path_entries", []):
        result.auto_path_entries.append(
            AutoPathEntry(
                resolved_path=None,
                raw=entry["raw"],
                range=range_at(*entry["range"]),
            )
        )

    return result


@dataclass
class _FactoryCandidate:
    """A 3-arg command call that might be a factory-wrapper proc definition.

    Tcllib's ``DEFC name args body`` / ``DEF name esc value`` pattern
    creates a proc at runtime via ``proc $name $args $body``.  We record
    every call with that shape and resolve it against the set of procs
    that actually behave as factories after the first pass finishes.
    """

    head: str  # the command name as written ("DEFC", "::foo::DEF", ...)
    name: str
    name_tok: Token
    body_tok: Token
    ns_prefix: str  # effective namespace at the call site (no leading ``::``)


@dataclass
class _ProcBodyInfo:
    """Tracking data for factory-wrapper detection."""

    qname: str
    params: list[str]
    body_text: str
    ns_prefix: str


@dataclass
class _ScanCtx:
    result: AnalysisResult
    candidates: list[_FactoryCandidate] = field(default_factory=list)
    proc_bodies: list[_ProcBodyInfo] = field(default_factory=list)


def extract_signatures(source: str) -> AnalysisResult:
    """Return a minimal AnalysisResult for a background-indexed file.

    Dispatches to the Rust port when ``TCL_LSP_RUST_SIGNATURE_SCAN``
    is set in the environment **and** the ``tcl_lsp_rust`` binding
    is importable; otherwise falls back to the Python implementation
    below. Any exception raised by the Rust path is logged at DEBUG
    and the Python path runs as a safety net — same fallback shape
    as `core.compiler.optimiser._manager`.
    """
    if _rust_extract is not None and os.environ.get("TCL_LSP_RUST_SIGNATURE_SCAN"):
        try:
            return _materialise_rust_signatures(source, _rust_extract(source))
        except Exception:  # pragma: no cover - safety net
            log.debug("rust signature_scan failed, falling back to python", exc_info=True)
    return _extract_signatures_python(source)


def _extract_signatures_python(source: str) -> AnalysisResult:
    """The original Python implementation. Kept as the default
    behaviour and as a fallback when the Rust path is disabled or
    fails."""
    ctx = _ScanCtx(result=AnalysisResult())
    _scan(source, body_token=None, ns_prefix="", conditional=False, ctx=ctx)
    _resolve_factory_defs(ctx)
    return ctx.result


def _qualify(ns_prefix: str, name: str) -> str:
    """Fully qualify *name* within *ns_prefix* following Tcl scoping.

    Absolute names (``::foo::bar``) ignore the prefix entirely so that a
    ``proc ::foo::bar`` declared inside ``namespace eval baz`` still
    indexes as ``::foo::bar``.
    """
    if name.startswith("::"):
        return name
    if ns_prefix:
        return f"::{ns_prefix}::{name}"
    return f"::{name}"


def _scan(
    source: str,
    body_token: Token | None,
    ns_prefix: str,
    conditional: bool,
    ctx: _ScanCtx,
) -> None:
    # ``recovery=True`` (segmenter default) — an unclosed brace or bracket
    # high up in the file must not swallow the rest of it and silently
    # drop every later declaration.
    commands = segment_commands(source, body_token=body_token)
    result = ctx.result
    for cmd in commands:
        if cmd.is_partial or not cmd.argv:
            continue
        texts = cmd.texts
        argv = cmd.argv
        head = texts[0] if texts else ""
        if not head:
            continue
        # Workspace usage counts rely on ``command_invocations``; emit a
        # lightweight record (name + range, no qualified-name resolution)
        # for every command so background-scanned files still contribute.
        result.command_invocations.append(
            CommandInvocation(name=head, range=range_from_token(argv[0]))
        )
        match head:
            case "proc":
                _handle_proc(texts, argv, ns_prefix, ctx)
            case "namespace":
                _handle_namespace(texts, argv, ns_prefix, conditional, ctx)
            case "package":
                _handle_package(texts, argv, conditional, result)
            case "source":
                _handle_source(texts, argv, result)
            case "interp":
                _handle_interp(texts, result)
            case "oo::class":
                _handle_oo_class(texts, argv, ns_prefix, result)
            case "itcl::class" | "::itcl::class":
                _handle_itcl_class(texts, argv, ns_prefix, result)
            case "if":
                _handle_if(texts, argv, ns_prefix, ctx)
            case "catch":
                _handle_catch(texts, argv, ns_prefix, ctx)
            case "try":
                _handle_try(texts, argv, ns_prefix, ctx)
            case "lappend" | "set":
                _handle_auto_path(head, texts, argv, result)
            case _:
                _maybe_handle_import_wrapper(head, texts, argv, ns_prefix, result)
                _maybe_record_factory_candidate(head, texts, argv, ns_prefix, ctx)


def _maybe_recurse_body(
    body_text: str,
    body_tok: Token,
    ns_prefix: str,
    conditional: bool,
    ctx: _ScanCtx,
) -> None:
    """Recurse into *body_tok* only when it is a braced script.

    Unbraced / substituted body arguments (``$body``, ``[gen_body]``)
    cannot be statically analysed, so we skip them rather than feed
    non-script text to the segmenter.
    """
    if body_tok.type is TokenType.STR:
        _scan(
            body_text,
            body_token=body_tok,
            ns_prefix=ns_prefix,
            conditional=conditional,
            ctx=ctx,
        )


def _handle_proc(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    ctx: _ScanCtx,
) -> None:
    if len(texts) < 4:
        return
    raw_name = texts[1]
    qualified = _qualify(ns_prefix, raw_name)
    simple = qualified.rsplit("::", 1)[-1]
    name_range = range_from_token(argv[1])
    body_range = range_from_token(argv[3]) if len(argv) > 3 else name_range
    params = parse_param_list(texts[2])
    ctx.result.all_procs[qualified] = ProcDef(
        name=simple,
        qualified_name=qualified,
        params=params,
        name_range=name_range,
        body_range=body_range,
    )
    # Record the body text so ``_resolve_factory_defs`` can identify
    # factory-wrapper procs (``proc $name $args $body`` idiom used by
    # tcllib's ``DEFC``).  The namespace the factory creates procs in
    # at call time is the one the wrapper itself belongs to, so keep
    # that around too.
    body_ns = qualified.rsplit("::", 1)[0].lstrip(":")
    ctx.proc_bodies.append(
        _ProcBodyInfo(
            qname=qualified,
            params=[p.name for p in params],
            body_text=texts[3],
            ns_prefix=body_ns,
        )
    )
    # Factory calls (``DEFC foo …``) typically live inside an ``INIT``
    # proc that populates a namespace at package-load time.  Walk the
    # body looking only for candidate call sites — we deliberately skip
    # the usual ``proc`` / ``namespace eval`` handlers here because
    # those definitions only take effect when the enclosing proc runs,
    # and recording them would confuse static resolution.
    if len(argv) > 3 and argv[3].type is TokenType.STR:
        _scan_factory_candidates(texts[3], argv[3], body_ns, ctx)


def _handle_namespace(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    conditional: bool,
    ctx: _ScanCtx,
) -> None:
    if len(texts) < 2:
        return
    sub = texts[1]
    if sub == "eval" and len(texts) >= 4:
        raw_ns = texts[2]
        # ``namespace eval ::foo {...}`` rebases the prefix rather than nesting
        # under the surrounding namespace — Tcl treats leading ``::`` as absolute.
        if raw_ns.startswith("::"):
            inner_prefix = raw_ns.lstrip(":")
        elif ns_prefix:
            inner_prefix = f"{ns_prefix}::{raw_ns}"
        else:
            inner_prefix = raw_ns
        _maybe_recurse_body(texts[3], argv[3], inner_prefix, conditional, ctx)
        return
    if sub == "import" and len(texts) >= 3:
        _handle_namespace_import(texts, argv, ns_prefix, ctx.result)


def _handle_namespace_import(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    # ``namespace import ?-force? pattern ?pattern ...?``
    importing_ns = f"::{ns_prefix}" if ns_prefix else "::"
    i = 2
    while i < len(texts):
        pattern = texts[i]
        if pattern == "-force":
            i += 1
            continue
        # Must be a qualified pattern — ``namespace import`` requires
        # a namespace segment; drop bare names or patterns that still
        # carry variable / command substitutions, because we cannot
        # statically resolve them to a source namespace.
        if "::" not in pattern or "$" in pattern or "[" in pattern:
            i += 1
            continue
        # A pattern without a leading ``::`` is resolved relative to
        # the *current* namespace, not the global one — Tcl's own rule
        # for ``namespace import``.  So
        # ``namespace eval foo { namespace import bar::* }`` imports
        # ``::foo::bar::*``, not ``::bar::*``.
        if not pattern.startswith("::"):
            if importing_ns == "::":
                pattern = f"::{pattern}"
            else:
                pattern = f"{importing_ns}::{pattern}"
        result.namespace_imports.append(
            NamespaceImport(
                ns=importing_ns,
                pattern=pattern,
                range=range_from_token(argv[i]),
            )
        )
        i += 1


def _maybe_handle_import_wrapper(
    head: str,
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    """Recognise the tcllib ``<NS>::import <ALIAS>`` wrapper idiom.

    A common tcllib convention is a proc ``some::ns::import`` that
    ``uplevel``s ``namespace eval <alias> {namespace import some::ns::*}``.
    Statically detecting all such wrappers from their bodies is out of
    scope, but the *call* is unambiguous — its qualified name ends in
    ``::import`` and its single argument is a namespace identifier.
    Record a conjectured import so ``vt::showat`` can resolve to
    ``::term::ansi::send::showat`` when the user opens an example that
    uses it.
    """
    if not head.endswith("::import"):
        return
    if len(texts) < 2:
        return
    alias = texts[1]
    if not alias or "$" in alias or "[" in alias:
        return
    # Import wrapper typically takes a single namespace token as its
    # first argument; remaining tokens would be pattern filters we can't
    # evaluate statically, so we play it safe and only infer when the
    # call is ``head alias``.
    if len(texts) > 2:
        return
    source_ns = head[: -len("::import")]  # strip trailing ``::import``
    if not source_ns.startswith("::"):
        source_ns = f"::{source_ns}"
    # A relative alias lives under the current namespace — inside
    # ``namespace eval outer { some::ns::import vt }``, Tcl would
    # create ``::outer::vt``, not ``::vt``.
    if alias.startswith("::"):
        alias_ns = alias
    elif ns_prefix:
        alias_ns = f"::{ns_prefix}::{alias}"
    else:
        alias_ns = f"::{alias}"
    result.namespace_imports.append(
        NamespaceImport(
            ns=alias_ns,
            pattern=f"{source_ns}::*",
            range=range_from_token(argv[0]),
            conjectured=True,
        )
    )


def _handle_auto_path(
    head: str,
    texts: list[str],
    argv: list[Token],
    result: AnalysisResult,
) -> None:
    """Record ``lappend auto_path …`` / ``set auto_path …`` entries."""
    if len(texts) < 3:
        return
    if texts[1] != "auto_path":
        return
    # ``set auto_path <expr>`` assigns the whole list; ``lappend
    # auto_path <expr>`` appends.  Both forms: every word after the
    # variable name is a path element.
    for i in range(2, len(texts)):
        raw = texts[i]
        if not raw:
            continue
        result.auto_path_entries.append(
            AutoPathEntry(
                resolved_path=None,
                raw=raw,
                range=range_from_token(argv[i]),
            )
        )
    _ = head  # currently no need to distinguish; reserved for future use


def _handle_package(
    texts: list[str],
    argv: list[Token],
    conditional: bool,
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
            conditional=conditional,
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
    # ``interp alias SLAVE-PATH NAME TARGET-PATH TARGET ?ARG ...?``
    #
    # We only record aliases defined in the *local* interpreter. Both
    # slave and target paths must be the empty list (``{}`` → empty
    # string after ``_word_piece``) for the alias to affect command
    # resolution in the current workspace; ``interp alias child foo {}
    # puts`` installs ``foo`` inside a child interpreter and is not a
    # workspace-visible alias.
    if len(texts) < 6 or texts[1] != "alias":
        return
    if texts[2] != "" or texts[4] != "":
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


def _handle_if(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    ctx: _ScanCtx,
) -> None:
    """Descend into every branch of an ``if`` chain.

    Tcl's ``if`` takes the shape ``if EXPR ?then? BODY
    ?elseif EXPR ?then? BODY?... ?else? ?BODY?``. We alternate
    between expecting an expression and expecting a body, resetting the
    expectation whenever the optional ``then`` / ``elseif`` / ``else``
    keywords appear. Everything discovered below is marked *conditional*
    so guarded ``package require`` statements don't influence workspace
    Tcl-version upgrade as if they were unconditional.
    """
    i = 1
    expect_body = False
    while i < len(texts):
        word = texts[i]
        if word == "then":
            expect_body = True
            i += 1
            continue
        if word == "elseif":
            expect_body = False
            i += 1
            continue
        if word == "else":
            expect_body = True
            i += 1
            continue
        if expect_body:
            _maybe_recurse_body(texts[i], argv[i], ns_prefix, True, ctx)
            expect_body = False
        else:
            expect_body = True
        i += 1


def _handle_catch(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    ctx: _ScanCtx,
) -> None:
    # ``catch SCRIPT ?RESULTVAR? ?OPTIONSVAR?``
    if len(texts) < 2:
        return
    _maybe_recurse_body(texts[1], argv[1], ns_prefix, True, ctx)


def _handle_try(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    ctx: _ScanCtx,
) -> None:
    """Descend into the main body, each ``on``/``trap`` handler, and ``finally``.

    Handler clauses take the shape ``on CODE VARLIST BODY`` and
    ``trap PATTERN VARLIST BODY`` — four words whose body is at offset
    +3. A ``finally BODY`` clause is always last.
    """
    if len(texts) < 2:
        return
    _maybe_recurse_body(texts[1], argv[1], ns_prefix, True, ctx)
    i = 2
    while i < len(texts):
        clause = texts[i]
        if clause == "finally" and i + 1 < len(texts):
            _maybe_recurse_body(texts[i + 1], argv[i + 1], ns_prefix, True, ctx)
            return
        if clause in ("on", "trap") and i + 3 < len(texts):
            _maybe_recurse_body(texts[i + 3], argv[i + 3], ns_prefix, True, ctx)
            i += 4
        else:
            i += 1


_FACTORY_SKIP_HEADS = frozenset(
    {
        # Commands that incidentally take the shape ``HEAD NAME BRACED BRACED``
        # but definitely aren't proc factories — skip by name so they do not
        # pollute the candidate list.
        "proc",
        "namespace",
        "if",
        "switch",
        "while",
        "for",
        "foreach",
        "try",
        "catch",
        "eval",
        "apply",
        "expr",
        "uplevel",
        "upvar",
        "variable",
        "set",
        "lappend",
        "dict",
        "array",
        "string",
        "list",
        "lindex",
        "package",
        "source",
        "interp",
        "oo::class",
        "oo::define",
        "oo::objdefine",
        "method",
        "classmethod",
        "itcl::class",
        "::itcl::class",
    }
)


def _maybe_record_factory_candidate(
    head: str,
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    ctx: _ScanCtx,
) -> None:
    """Record a potential factory-wrapper call for post-scan resolution.

    A tcllib factory call has the shape ``HEAD NAME ARGS BODY`` — four
    tokens total where the last is a braced script.  We defer deciding
    whether HEAD is a real factory until after the full scan, so filter
    here only on the structural shape and a negative list of built-ins
    that happen to share it.
    """
    if len(texts) != 4:
        return
    if head in _FACTORY_SKIP_HEADS:
        return
    # Name argument must be a plain word — variable and command
    # substitutions mean the created proc has a runtime-computed name we
    # cannot statically resolve.
    name = texts[1]
    if not name or "$" in name or "[" in name:
        return
    body_tok = argv[3]
    if body_tok.type is not TokenType.STR:
        return
    ctx.candidates.append(
        _FactoryCandidate(
            head=head,
            name=name,
            name_tok=argv[1],
            body_tok=body_tok,
            ns_prefix=ns_prefix,
        )
    )


def _scan_factory_candidates(
    body_text: str,
    body_tok: Token,
    ns_prefix: str,
    ctx: _ScanCtx,
) -> None:
    """Record factory-candidate calls inside a proc body.

    Unlike :func:`_scan`, this pass walks a proc's body looking only
    for ``HEAD NAME {ARGS} {BODY}`` shaped calls and descends into
    structural-control bodies that commonly contain factory calls
    (``if``/``catch``/``try``/``namespace eval``).  It deliberately
    does not emit nested proc defs or namespace imports — those would
    be incorrect because nested ``proc`` statements inside a proc body
    only take effect when that proc is invoked.
    """
    try:
        commands = segment_commands(body_text, body_token=body_tok)
    except Exception:  # pragma: no cover - defensive
        return
    for cmd in commands:
        if cmd.is_partial or not cmd.argv:
            continue
        texts = cmd.texts
        argv = cmd.argv
        head = texts[0] if texts else ""
        if not head:
            continue
        if head in ("if", "catch", "try", "namespace"):
            # Descend into braced bodies that commonly wrap factory calls.
            _scan_factory_structural(head, texts, argv, ns_prefix, ctx)
            continue
        _maybe_record_factory_candidate(head, texts, argv, ns_prefix, ctx)


def _scan_factory_structural(
    head: str,
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    ctx: _ScanCtx,
) -> None:
    """Recurse a structural command's braced bodies for factory candidates."""
    if head == "namespace" and len(texts) >= 4 and texts[1] == "eval":
        raw_ns = texts[2]
        if raw_ns.startswith("::"):
            inner = raw_ns.lstrip(":")
        elif ns_prefix:
            inner = f"{ns_prefix}::{raw_ns}"
        else:
            inner = raw_ns
        if argv[3].type is TokenType.STR:
            _scan_factory_candidates(texts[3], argv[3], inner, ctx)
        return
    if head == "if":
        i = 1
        expect_body = False
        while i < len(texts):
            w = texts[i]
            if w == "then":
                expect_body = True
                i += 1
                continue
            if w == "elseif":
                expect_body = False
                i += 1
                continue
            if w == "else":
                expect_body = True
                i += 1
                continue
            if expect_body and argv[i].type is TokenType.STR:
                _scan_factory_candidates(texts[i], argv[i], ns_prefix, ctx)
                expect_body = False
            else:
                expect_body = True
            i += 1
        return
    if head == "catch" and len(texts) >= 2 and argv[1].type is TokenType.STR:
        _scan_factory_candidates(texts[1], argv[1], ns_prefix, ctx)
        return
    if head == "try" and len(texts) >= 2:
        if argv[1].type is TokenType.STR:
            _scan_factory_candidates(texts[1], argv[1], ns_prefix, ctx)
        i = 2
        while i < len(texts):
            clause = texts[i]
            if clause == "finally" and i + 1 < len(texts) and argv[i + 1].type is TokenType.STR:
                _scan_factory_candidates(texts[i + 1], argv[i + 1], ns_prefix, ctx)
                return
            if (
                clause in ("on", "trap")
                and i + 3 < len(texts)
                and argv[i + 3].type is TokenType.STR
            ):
                _scan_factory_candidates(texts[i + 3], argv[i + 3], ns_prefix, ctx)
                i += 4
            else:
                i += 1
        return


def _is_factory_body(body_text: str, params: list[str]) -> bool:
    """Return True when *body_text* matches the ``proc $p1 $p2 $p3`` idiom.

    The canonical tcllib wrapper is::

        proc foo {name args body} {
            ...
            proc $name $args $body
            ...
        }

    We look for any top-level ``proc`` command whose three arguments
    are ``$name $args $body`` in some order matching the wrapper's own
    parameter list.  Non-variable arguments disqualify the match to
    avoid treating ``proc $name foo body`` as a factory.
    """
    if len(params) < 3:
        return False
    try:
        commands = segment_commands(body_text)
    except Exception:  # pragma: no cover - defensive
        return False
    # The segmenter reconstructs substitutions as ``${name}``; accept the
    # bare ``$name`` form too for robustness.
    param_vars = set()
    for p in params:
        param_vars.add(f"${p}")
        param_vars.add(f"${{{p}}}")
    for cmd in commands:
        if cmd.is_partial or not cmd.texts:
            continue
        t = cmd.texts
        if t[0] != "proc" or len(t) != 4:
            continue
        if t[1] in param_vars and t[2] in param_vars and t[3] in param_vars:
            return True
    return False


def _resolve_factory_defs(ctx: _ScanCtx) -> None:
    """Emit synthetic ProcDefs for each factory-wrapper call site.

    Factory wrappers are looked up following Tcl's command-resolution
    order: an unqualified ``DEFC`` inside namespace ``::ns`` binds to
    ``::ns::DEFC`` first, then falls back to ``::DEFC`` (global).  We
    deliberately do *not* fall back to "any factory named DEFC
    anywhere" — two unrelated namespaces can each define their own
    ``DEFC`` wrapper and Tcl itself would refuse to cross those
    boundaries, so we should not emit synthetic procs in the wrong
    namespace.
    """
    if not ctx.candidates or not ctx.proc_bodies:
        return
    # Map every factory wrapper's qualified name to the namespace its
    # generated procs live in (the wrapper's own home namespace —
    # ``proc $name …`` executed inside a proc creates the command in
    # the caller's current namespace, which for a factory wrapper
    # defined in ``::foo::bar`` and called from an init proc *also* in
    # ``::foo::bar`` is unambiguously ``::foo::bar``).
    factories: dict[str, str] = {}
    for info in ctx.proc_bodies:
        if not _is_factory_body(info.body_text, info.params):
            continue
        factories[info.qname] = info.ns_prefix
    if not factories:
        return
    for cand in ctx.candidates:
        factory_qname = _lookup_factory(cand, factories)
        if factory_qname is None:
            continue
        factory_ns = factories[factory_qname]
        # The emitted proc lives in the factory's own namespace unless
        # the call supplied a fully-qualified name.
        if cand.name.startswith("::"):
            emitted_q = cand.name
        elif factory_ns:
            emitted_q = f"::{factory_ns}::{cand.name}"
        else:
            emitted_q = f"::{cand.name}"
        simple = emitted_q.rsplit("::", 1)[-1]
        if emitted_q in ctx.result.all_procs:
            continue
        ctx.result.all_procs[emitted_q] = ProcDef(
            name=simple,
            qualified_name=emitted_q,
            params=[],  # factory callers supply params but we'd need
            # the wrapper's mapping to know which token corresponds;
            # leave empty — users still get a jump target.
            name_range=range_from_token(cand.name_tok),
            body_range=range_from_token(cand.body_tok),
        )


def _lookup_factory(
    cand: _FactoryCandidate,
    factories: dict[str, str],
) -> str | None:
    """Resolve *cand.head* to a factory qualified name.

    Walks Tcl's command lookup path: absolute names match verbatim;
    relative names try the call-site namespace first, then the global
    namespace.  Never falls through to "any factory with this bare
    name" — that would bind calls in one namespace to a wrapper in an
    unrelated one.
    """
    head = cand.head
    if head.startswith("::"):
        return head if head in factories else None
    qualified = _qualify(cand.ns_prefix, head)
    if qualified in factories:
        return qualified
    global_q = f"::{head}"
    if global_q != qualified and global_q in factories:
        return global_q
    return None


def _emit_class(
    raw_name: str,
    name_tok: Token,
    body_tok: Token,
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    # Don't strip ``::`` — ``_qualify`` needs to see the leading colons
    # to treat ``oo::class create ::Foo`` inside a namespace as absolute
    # and index it as ``::Foo`` rather than ``::ns::Foo``.
    qualified = _qualify(ns_prefix, raw_name)
    simple = qualified.rsplit("::", 1)[-1]
    result.all_classes[qualified] = ClassDef(
        name=simple,
        qualified_name=qualified,
        name_range=range_from_token(name_tok),
        body_range=range_from_token(body_tok),
    )
