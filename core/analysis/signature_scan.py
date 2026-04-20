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

from core.analysis.analyser import parse_param_list
from core.analysis.semantic_model import (
    AnalysisResult,
    ClassDef,
    CommandInvocation,
    PackageRequire,
    ProcDef,
    SourceTarget,
)
from core.common.ranges import range_from_token
from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import Token, TokenType


def extract_signatures(source: str) -> AnalysisResult:
    """Return a minimal AnalysisResult for a background-indexed file."""
    result = AnalysisResult()
    _scan(source, body_token=None, ns_prefix="", conditional=False, result=result)
    return result


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
    result: AnalysisResult,
) -> None:
    # ``recovery=True`` (segmenter default) — an unclosed brace or bracket
    # high up in the file must not swallow the rest of it and silently
    # drop every later declaration.
    commands = segment_commands(source, body_token=body_token)
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
                _handle_proc(texts, argv, ns_prefix, result)
            case "namespace":
                _handle_namespace(texts, argv, ns_prefix, conditional, result)
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
                _handle_if(texts, argv, ns_prefix, result)
            case "catch":
                _handle_catch(texts, argv, ns_prefix, result)
            case "try":
                _handle_try(texts, argv, ns_prefix, result)


def _maybe_recurse_body(
    body_text: str,
    body_tok: Token,
    ns_prefix: str,
    conditional: bool,
    result: AnalysisResult,
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
            result=result,
        )


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
        params=parse_param_list(texts[2]),
        name_range=name_range,
        body_range=body_range,
    )


def _handle_namespace(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    conditional: bool,
    result: AnalysisResult,
) -> None:
    if len(texts) < 4 or texts[1] != "eval":
        return
    raw_ns = texts[2]
    # ``namespace eval ::foo {...}`` rebases the prefix rather than nesting
    # under the surrounding namespace — Tcl treats leading ``::`` as absolute.
    if raw_ns.startswith("::"):
        inner_prefix = raw_ns.lstrip(":")
    elif ns_prefix:
        inner_prefix = f"{ns_prefix}::{raw_ns}"
    else:
        inner_prefix = raw_ns
    _maybe_recurse_body(texts[3], argv[3], inner_prefix, conditional, result)


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
    result: AnalysisResult,
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
            _maybe_recurse_body(texts[i], argv[i], ns_prefix, True, result)
            expect_body = False
        else:
            expect_body = True
        i += 1


def _handle_catch(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    # ``catch SCRIPT ?RESULTVAR? ?OPTIONSVAR?``
    if len(texts) < 2:
        return
    _maybe_recurse_body(texts[1], argv[1], ns_prefix, True, result)


def _handle_try(
    texts: list[str],
    argv: list[Token],
    ns_prefix: str,
    result: AnalysisResult,
) -> None:
    """Descend into the main body, each ``on``/``trap`` handler, and ``finally``.

    Handler clauses take the shape ``on CODE VARLIST BODY`` and
    ``trap PATTERN VARLIST BODY`` — four words whose body is at offset
    +3. A ``finally BODY`` clause is always last.
    """
    if len(texts) < 2:
        return
    _maybe_recurse_body(texts[1], argv[1], ns_prefix, True, result)
    i = 2
    while i < len(texts):
        clause = texts[i]
        if clause == "finally" and i + 1 < len(texts):
            _maybe_recurse_body(texts[i + 1], argv[i + 1], ns_prefix, True, result)
            return
        if clause in ("on", "trap") and i + 3 < len(texts):
            _maybe_recurse_body(texts[i + 3], argv[i + 3], ns_prefix, True, result)
            i += 4
        else:
            i += 1


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
