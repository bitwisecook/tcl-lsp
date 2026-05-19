"""Built-in ensemble namespace registration.

C Tcl 9.0 ``Tcl_CreateInterp`` populates a fixed set of implementation
namespaces (``::tcl::string``, ``::tcl::dict``, ``::tcl::info``, …) with
one command per ensemble subcommand (``::tcl::string::length``, etc.).
Tests that use ``info commands ::tcl::string::*``, ``namespace exists
::tcl::dict``, or ``namespace import ::tcl::string::*`` depend on those
namespaces being present from the moment the interp comes up.

(The math namespaces ``::tcl::mathfunc`` and ``::tcl::mathop`` are
populated separately — see ``math_cmds.py`` and the runtime/Zig
backends — so any motivating ``namespace import ::tcl::mathop::*`` use
case is covered there, not here.)

The bytecode interpreter already re-dispatches qualified ensemble names
through ``_FQ_ENSEMBLE_RE`` in ``interp.py`` (``::tcl::dict::get`` →
``dict get``), so execution works without these stubs.  This module
exists so that *introspection* matches C Tcl: it creates each
implementation namespace, registers a thin alias handler for every
subcommand, and adds a ``*`` export pattern so ``namespace import``
finds them.

Subcommand lists come from the C tables in ``tcl9.0.3/generic/``:
``stringImplMap`` (``tclCmdMZ.c``), ``arrayImplMap`` (``tclVar.c``),
``binaryMap`` / ``encodeMap`` / ``decodeMap`` (``tclBinary.c``), the
``chan`` ``initMap`` (``tclIOCmd.c``), ``ClockCommand`` table
(``tclClock.c``), ``implementationMap`` (``tclDictObj.c``),
``encodingImplMap`` and ``file`` ``initMap`` (``tclCmdAH.c``),
``defaultInfoMap`` (``tclCmdIL.c``), ``defaultNamespaceMap``
(``tclNamesp.c``), ``prefixImplMap`` (``tclIndexObj.c``),
``processImplMap`` (``tclProcess.c``), ``zlibImplMap`` (``tclZlib.c``),
the ``zipfs`` ensemble (``tclZipfs.c``), and the OO tables in
``tclOO.c`` / ``tclOOInfo.c``.

Mathfunc and mathop are *not* handled here — they are owned by
``math_cmds.py`` and the runtime/Zig backends respectively.

bcc-shape probes — ``::tcl::unsupported::representation``,
``disassemble``, ``getbytecode``, ``assemble`` — describe the C
interpreter's ``Tcl_Obj`` layout and bytecode listing.  Neither the
Python VM nor the Zig WASM runtime produces that shape, so faking the
output would mislead pattern-matching tests.  We register the names
(matching C ``info commands`` introspection) but the handlers raise a
clear "not implemented in this runtime" error; tests that exercise
them are categorised under the ``skip`` bucket in
``tests/baselines/tcl9-tcltest/categories/<stem>.toml`` and never run.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..scope import ensure_namespace
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp

# Subcommand tables — bare names only.  Each entry becomes
# ``::tcl::<ns>::<sub>`` registered inside the ``::tcl::<ns>`` namespace
# and dispatched back through the bare ensemble (``<ns> <sub> …``).

_ARRAY_SUBCMDS = (
    "anymore",
    "default",
    "donesearch",
    "exists",
    "for",
    "get",
    "names",
    "nextelement",
    "set",
    "size",
    "startsearch",
    "statistics",
    "unset",
)

_BINARY_SUBCMDS = ("format", "scan", "encode", "decode")
_BINARY_ENCODE_SUBCMDS = ("hex", "uuencode", "base64")
_BINARY_DECODE_SUBCMDS = ("hex", "uuencode", "base64")

_CHAN_SUBCMDS = (
    "blocked",
    "close",
    "configure",
    "copy",
    "create",
    "eof",
    "event",
    "flush",
    "gets",
    "isbinary",
    "names",
    "pending",
    "pipe",
    "pop",
    "postevent",
    "push",
    "puts",
    "read",
    "seek",
    "tell",
    "truncate",
)

# Includes the user-facing commands plus the C-internal helpers, so
# that ``info commands ::tcl::clock::*`` enumerates the same names as
# C Tcl.  Calls to unimplemented helpers fall through to the bare
# ``clock`` ensemble, which produces the standard "bad subcommand"
# error.
_CLOCK_SUBCMDS = (
    "add",
    "catch",
    "clicks",
    "ConvertLocalToUTC",
    "format",
    "GetDateFields",
    "GetJulianDayFromEraYearMonthDay",
    "GetJulianDayFromEraYearWeekDay",
    "getenv",
    "microseconds",
    "milliseconds",
    "scan",
    "seconds",
)

_DICT_SUBCMDS = (
    "append",
    "create",
    "exists",
    "filter",
    "for",
    "get",
    "getdef",
    "getwithdefault",
    "incr",
    "info",
    "keys",
    "lappend",
    "map",
    "merge",
    "remove",
    "replace",
    "set",
    "size",
    "unset",
    "update",
    "values",
    "with",
)

_ENCODING_SUBCMDS = (
    "convertfrom",
    "convertto",
    "dirs",
    "names",
    "profiles",
    "system",
    "user",
)

_FILE_SUBCMDS = (
    "atime",
    "attributes",
    "channels",
    "copy",
    "delete",
    "dirname",
    "executable",
    "exists",
    "extension",
    "home",
    "isdirectory",
    "isfile",
    "join",
    "link",
    "lstat",
    "mtime",
    "mkdir",
    "nativename",
    "normalize",
    "owned",
    "pathtype",
    "readable",
    "readlink",
    "rename",
    "rootname",
    "separator",
    "size",
    "split",
    "stat",
    "system",
    "tail",
    "tempdir",
    "tempfile",
    "tildeexpand",
    "type",
    "volumes",
    "writable",
)

_INFO_SUBCMDS = (
    "args",
    "body",
    "cmdcount",
    "cmdtype",
    "commands",
    "complete",
    "constant",
    "consts",
    "coroutine",
    "default",
    "errorstack",
    "exists",
    "frame",
    "functions",
    "globals",
    "hostname",
    "level",
    "library",
    "loaded",
    "locals",
    "nameofexecutable",
    "patchlevel",
    "procs",
    "script",
    "sharedlibextension",
    "tclversion",
    "vars",
)

_NAMESPACE_SUBCMDS = (
    "children",
    "code",
    "current",
    "delete",
    "ensemble",
    "eval",
    "exists",
    "export",
    "forget",
    "import",
    "inscope",
    "origin",
    "parent",
    "path",
    "qualifiers",
    "tail",
    "unknown",
    "upvar",
    "which",
)

_PREFIX_SUBCMDS = ("all", "longest", "match")

_PROCESS_SUBCMDS = ("autopurge", "list", "purge", "status")

_STRING_SUBCMDS = (
    "cat",
    "compare",
    "equal",
    "first",
    "index",
    "insert",
    "is",
    "last",
    "length",
    "map",
    "match",
    "range",
    "repeat",
    "replace",
    "reverse",
    "tolower",
    "totitle",
    "toupper",
    "trim",
    "trimleft",
    "trimright",
    "wordend",
    "wordstart",
)

_ZLIB_SUBCMDS = (
    "adler32",
    "compress",
    "crc32",
    "decompress",
    "deflate",
    "gunzip",
    "gzip",
    "inflate",
    "push",
    "stream",
)

_ZIPFS_SUBCMDS = (
    "canonical",
    "exists",
    "find",
    "info",
    "list",
    "lmkimg",
    "lmkzip",
    "mkimg",
    "mkkey",
    "mkzip",
    "mount",
    "mountdata",
    "root",
    "unmount",
)

# (namespace, bare-ensemble, subcommand-tuple) — every entry produces
# ``::tcl::<ns>`` populated with aliases that re-dispatch as
# ``<bare> <sub> …``.  The bare ensemble is what the existing
# command handler is registered as on the global REGISTRY.
_ENSEMBLE_NAMESPACES: tuple[tuple[str, str, tuple[str, ...]], ...] = (
    ("::tcl::array", "array", _ARRAY_SUBCMDS),
    ("::tcl::binary", "binary", _BINARY_SUBCMDS),
    ("::tcl::chan", "chan", _CHAN_SUBCMDS),
    ("::tcl::clock", "clock", _CLOCK_SUBCMDS),
    ("::tcl::dict", "dict", _DICT_SUBCMDS),
    ("::tcl::encoding", "encoding", _ENCODING_SUBCMDS),
    ("::tcl::file", "file", _FILE_SUBCMDS),
    ("::tcl::info", "info", _INFO_SUBCMDS),
    ("::tcl::namespace", "namespace", _NAMESPACE_SUBCMDS),
    ("::tcl::prefix", "prefix", _PREFIX_SUBCMDS),
    ("::tcl::process", "process", _PROCESS_SUBCMDS),
    ("::tcl::string", "string", _STRING_SUBCMDS),
    ("::tcl::zlib", "zlib", _ZLIB_SUBCMDS),
    ("::tcl::zipfs", "zipfs", _ZIPFS_SUBCMDS),
)

# Sub-ensembles whose parent is itself an ensemble.  ``binary encode``
# and ``binary decode`` route through ``binary <sub> …``.
_SUB_ENSEMBLE_NAMESPACES: tuple[tuple[str, tuple[str, ...], tuple[str, ...]], ...] = (
    ("::tcl::binary::encode", ("binary", "encode"), _BINARY_ENCODE_SUBCMDS),
    ("::tcl::binary::decode", ("binary", "decode"), _BINARY_DECODE_SUBCMDS),
)

# OO helper namespaces.  ``::oo::Helpers`` holds in-method utilities
# (``next``, ``self``, …) that the parser injects when compiling a
# method body.  The Info{Object,Class} ensembles are introspection
# helpers reached via ``info object`` / ``info class``.
_OO_HELPERS_SUBCMDS = (
    "callback",
    "classvariable",
    "link",
    "mymethod",
    "next",
    "nextto",
    "self",
)

_OO_INFO_OBJECT_SUBCMDS = (
    "call",
    "class",
    "creationid",
    "definition",
    "filters",
    "forward",
    "isa",
    "methods",
    "methodtype",
    "mixins",
    "namespace",
    "properties",
    "variables",
    "vars",
)

_OO_INFO_CLASS_SUBCMDS = (
    "call",
    "constructor",
    "definition",
    "definitionnamespace",
    "destructor",
    "filters",
    "forward",
    "instances",
    "methods",
    "methodtype",
    "mixins",
    "properties",
    "subclasses",
    "superclasses",
    "variables",
)


def _make_alias(bare: str, sub: str):
    """Build a thin handler that redispatches as ``<bare> <sub> …``."""

    def handler(interp: "TclInterp", args: list[str]) -> TclResult:
        return interp.invoke(bare, [sub] + list(args))

    return handler


def _make_sub_alias(parent_bare: str, parent_sub: str, sub: str):
    """Two-level redispatch for ``binary encode hex`` and friends."""

    def handler(interp: "TclInterp", args: list[str]) -> TclResult:
        return interp.invoke(parent_bare, [parent_sub, sub] + list(args))

    return handler


def _make_oo_info_alias(category: str, sub: str):
    """Redispatch ``::oo::InfoObject::<sub>`` through ``info object <sub>``.

    C Tcl exposes ``info object …`` and ``info class …`` as
    sub-ensembles attached to ``info``.  We mirror this so that
    ``::oo::InfoObject::class $obj`` reaches the same handler as
    ``info object class $obj``.
    """

    def handler(interp: "TclInterp", args: list[str]) -> TclResult:
        return interp.invoke("info", [category, sub] + list(args))

    return handler


def _register_namespace(
    interp: "TclInterp",
    qualname: str,
    subcmds: tuple[str, ...],
    handler_factory,
) -> None:
    ns = ensure_namespace(interp.root_namespace, qualname)
    for sub in subcmds:
        ns.register_command(sub, handler_factory(sub))
    ns.add_export("*")
    # NB: do *not* duplicate the alias into ``interp._runtime_commands``
    # under the FQN.  A flat-table copy would shadow the namespace
    # entry and survive ``ns.delete()`` / ``rename`` operations,
    # leaving ``::tcl::dict::get`` callable after ``namespace delete
    # ::tcl::dict``.  Dispatch already resolves qualified names via
    # the namespace path in ``_invoke_inner`` (line ~570), so the
    # namespace registration alone is sufficient.


def _bgerror_default(interp: "TclInterp", args: list[str]) -> TclResult:
    """Default ``::tcl::Bgerror`` handler — mirrors C Tcl behaviour.

    C Tcl registers ``TclDefaultBgErrorHandlerObjCmd`` here.  When no
    application-supplied ``bgerror`` is in scope, the runtime falls
    back to this and writes the error message and stack trace to
    stderr.  Our minimal version preserves the behaviour: print the
    message + errorInfo and return empty.

    Signature matches C: exactly two positional arguments, ``message``
    and ``returnOptions`` — the latter is the ``catch -returnOptions``
    dict from which we pull ``-errorinfo``.
    """
    if len(args) != 2:
        raise TclError('wrong # args: should be "::tcl::Bgerror message returnOptions"')
    import sys

    message = args[0]
    # The second arg (return-options dict) is informational — C Tcl
    # uses it to extract -errorinfo.  We pull the same key when
    # present, otherwise fall back to interp's errorInfo.
    error_info = ""
    from ..machine import _split_list

    try:
        opts = _split_list(args[1])
        for i in range(0, len(opts) - 1, 2):
            if opts[i] == "-errorinfo":
                error_info = opts[i + 1]
                break
    except TclError:
        pass
    if not error_info:
        error_info = interp.global_frame.get_var("errorInfo", default="")
    sys.stderr.write(f"{message}\n")
    if error_info:
        sys.stderr.write(f"{error_info}\n")
    return TclResult(value="")


def _bcc_shape_unsupported(cmd: str):
    """Build a handler that rejects bcc-bytecode-shape introspection.

    ``::tcl::unsupported::representation``, ``disassemble``, and
    ``getbytecode`` are bcc-internal probes — they describe the C
    interpreter's ``Tcl_Obj`` layout and bytecode listing.  Neither
    the Python VM nor the WASM Zig runtime emits that shape, so any
    response we produced would either lie about C-Tcl internals or
    be parsed as garbage by callers expecting the bcc format.

    Tests that depend on these commands are categorised under the
    ``skip`` bucket in
    ``tests/baselines/tcl9-tcltest/categories/<stem>.toml`` (see
    that directory's README) — the harness injects
    ``tcltest::configure -skip`` so the offending IDs never run.
    Outside that path, callers that probe via
    ``[info commands ::tcl::unsupported::representation]`` still see
    the command exist (matching C), and direct invocation produces
    the honest error below rather than a fabricated string.
    """

    def handler(interp: "TclInterp", args: list[str]) -> TclResult:
        raise TclError(
            f"::tcl::unsupported::{cmd} is not implemented in this "
            "runtime: it inspects C-bcc Tcl_Obj/bytecode shape that "
            "neither the Python VM nor the Zig WASM backend produces"
        )

    return handler


def _unsupported_assemble(interp: "TclInterp", args: list[str]) -> TclResult:
    """``::tcl::unsupported::assemble`` — accepts bcc bytecode source.

    Same reasoning as :func:`_bcc_shape_unsupported`: the assembler
    parses C bcc instruction mnemonics that our codegen does not
    emit, regardless of whether the host is the Python VM or the
    WASM Zig runtime.  ``assemble.test`` is currently marked
    ``trap_allowed`` at the stem level (see
    ``categories/assemble.toml``).
    """
    raise TclError(
        "::tcl::unsupported::assemble is not implemented in this "
        "runtime: it produces C-bcc bytecode that neither the Python "
        "VM nor the Zig WASM backend executes"
    )


def _unsupported_loadIcu(interp: "TclInterp", args: list[str]) -> TclResult:
    """``::tcl::unsupported::loadIcu`` — ICU is not bundled."""
    raise TclError("ICU support not available")


def _unsupported_clock_configure(interp: "TclInterp", args: list[str]) -> TclResult:
    """``::tcl::unsupported::clock::configure`` — accept ``-init-complete``.

    init.tcl invokes this with ``-init-complete`` once the ``clock``
    ensemble has been wired up.  Other flags are no-ops here — we
    have no per-interp clock state to configure.
    """
    return TclResult(value="")


def setup_ensemble_namespaces(interp: "TclInterp") -> None:
    """Create ``::tcl::*`` and ``::oo::*`` implementation namespaces.

    Idempotent — safe to call multiple times on the same interp.
    """
    if getattr(interp, "_ensemble_namespaces_setup", False):
        return
    interp._ensemble_namespaces_setup = True

    # ``::tcl`` itself — implicitly created as a parent of every
    # ``::tcl::<ns>``, but call out the ensure here so it exists even
    # if every sub-ensemble registration is later skipped.
    ensure_namespace(interp.root_namespace, "::tcl")

    for qualname, bare, subcmds in _ENSEMBLE_NAMESPACES:
        _register_namespace(
            interp,
            qualname,
            subcmds,
            lambda sub, _bare=bare: _make_alias(_bare, sub),
        )

    for qualname, (parent_bare, parent_sub), subcmds in _SUB_ENSEMBLE_NAMESPACES:
        _register_namespace(
            interp,
            qualname,
            subcmds,
            lambda sub, _pb=parent_bare, _ps=parent_sub: _make_sub_alias(_pb, _ps, sub),
        )

    # OO helpers + Info{Object,Class}.  ``::oo`` is already created
    # in TclInterp.__init__; ``configuresupport`` is namespace-only.
    _register_namespace(
        interp,
        "::oo::Helpers",
        _OO_HELPERS_SUBCMDS,
        lambda sub: _oo_helpers_stub(sub),
    )
    _register_namespace(
        interp,
        "::oo::InfoObject",
        _OO_INFO_OBJECT_SUBCMDS,
        lambda sub: _make_oo_info_alias("object", sub),
    )
    _register_namespace(
        interp,
        "::oo::InfoClass",
        _OO_INFO_CLASS_SUBCMDS,
        lambda sub: _make_oo_info_alias("class", sub),
    )
    ensure_namespace(interp.root_namespace, "::oo::configuresupport")
    ensure_namespace(interp.root_namespace, "::oo::configuresupport::objectinternal")
    ensure_namespace(interp.root_namespace, "::oo::configuresupport::classinternal")

    # ``::tcl::unsupported`` — singleton commands.
    unsupported_ns = ensure_namespace(interp.root_namespace, "::tcl::unsupported")
    unsupported_cmds = {
        "representation": _bcc_shape_unsupported("representation"),
        "disassemble": _bcc_shape_unsupported("disassemble"),
        "getbytecode": _bcc_shape_unsupported("getbytecode"),
        "assemble": _unsupported_assemble,
        "loadIcu": _unsupported_loadIcu,
    }
    for sub, handler in unsupported_cmds.items():
        unsupported_ns.register_command(sub, handler)
    unsupported_ns.add_export("*")

    # init.tcl pokes ``::tcl::unsupported::clock::configure -init-complete``
    # immediately after wiring up the user-facing ``clock`` ensemble.
    unsupported_clock_ns = ensure_namespace(interp.root_namespace, "::tcl::unsupported::clock")
    unsupported_clock_ns.register_command("configure", _unsupported_clock_configure)

    # ``::tcl::Bgerror`` — singleton command at ``::tcl`` level, not
    # inside a sub-namespace.  (``::tcl::build-info`` is registered
    # separately by ``runtime/zig/cmds/inspect.zig`` /
    # ``core/.../tcl_build_info.py``; we don't duplicate it here.)
    tcl_ns = ensure_namespace(interp.root_namespace, "::tcl")
    tcl_ns.register_command("Bgerror", _bgerror_default)


def _oo_helpers_stub(sub: str):
    """``::oo::Helpers`` aliases.

    These commands are only meaningful inside a method body — outside
    of one they raise the same error C Tcl would.  Concrete dispatch
    happens in ``oo_cmds.py`` when a method frame is active; this stub
    only ensures the names exist for ``info commands ::oo::Helpers::*``
    and ``namespace import ::oo::Helpers::*``.
    """

    def handler(interp: "TclInterp", args: list[str]) -> TclResult:
        # If we're inside a method, the bare command is registered in
        # the runtime table — defer to that.
        bare = interp.lookup_command(sub)
        if bare is not None:
            return bare(interp, args)
        raise TclError(f'invalid command name "{sub}": only valid inside a method')

    return handler
