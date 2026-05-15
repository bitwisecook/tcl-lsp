"""``f5 query`` — jq-flavoured DSL for inspecting and rewriting BIG-IP configs.

The grammar, value model, and builtin library all live in
:mod:`core.bigip.query`; this module is the argparse plumbing and the
custom help actions that surface the DSL reference, builtin catalogue,
and worked-example cookbook on the terminal.

Help layers:

- ``--help`` (the default) — verb summary, flag table, and pointers to
  the deeper help.  ``RawDescriptionHelpFormatter`` keeps the example
  block readable.
- ``--help-dsl`` — prints :func:`core.bigip.query.format_grammar`, the
  same grammar reference that lives in ``docs/design/f5-query-dsl.md``.
- ``--help-builtins [NAME]`` — full builtin catalogue, or just one
  function when a name is given.  Generated from the same registry the
  evaluator dispatches against, so docs and code cannot drift.
- ``--help-examples`` — :func:`core.bigip.query.format_examples` —
  a cookbook of common one-liners.  Every example is also exercised
  by ``tests/test_f5_query.py`` so a broken example fails CI before it
  ever ships.
"""

from __future__ import annotations

import argparse
import difflib
import json
import sys
from pathlib import Path

from core.bigip.query import (
    format_builtins,
    format_examples,
    format_grammar,
    run_query,
)
from core.bigip.query.errors import QueryError
from core.bigip.query.output import render

from ._emit import add_format_arg, render_config
from ._paths import read_path
from ._registry import verb

_DESCRIPTION = (
    "Inspect and rewrite BIG-IP configuration with a small jq-flavoured "
    "DSL.  Queries navigate the parsed object tree (``.ltm.virtual[]``, "
    '``.ltm.pool["/Common/web_pool"]``, ``.ltm.rule[]``), filter with '
    "``select(...)``, project fields, and — with ``=`` / ``|=`` / "
    "``+=`` / ``-=`` — rewrite matched values.  Identity-field "
    "writes auto-route through the same engine ``f5 rename`` uses, so "
    "renaming a pool also moves every reference to it.\n"
    "\n"
    "Default behaviour is a dry-run preview: a unified diff for "
    "mutating queries, or the projected values for read-only ones.  "
    "Pass ``--write`` to print the rewritten config to stdout, "
    "``--in-place`` to overwrite the input.\n"
)


_EPILOG = (
    "Examples:\n"
    "  # List every VS's default pool\n"
    "  f5 query '.ltm.virtual[] | .pool' bigip.conf\n"
    "\n"
    "  # VSes whose pool member is in 10.0.0.0/8\n"
    "  f5 query '.ltm.virtual[] | select(any(.pool.members[].address "
    '| in_cidr(., "10.0.0.0/8"))) | .name\' bigip.conf\n'
    "\n"
    "  # Readdress every VS into 192.168.9.0/24 (dry-run diff)\n"
    "  f5 query '.ltm.virtual[] | .destination |= ip(\"192.168.9.0/24\", .)' "
    "bigip.conf\n"
    "\n"
    "  # Rename a pool everywhere (header + references)\n"
    '  f5 query \'.ltm.pool["/Common/old"].name = "/Common/new"\' '
    "--write bigip.conf > new.conf\n"
    "\n"
    "Multi-config (cross-reference GTM + LTM, multiple LTM tiers, ...):\n"
    "  # Auto-named (filename stem): $ltm and $gtm bind automatically.\n"
    "  f5 query '$ltm.ltm.virtual[].name' gtm.conf ltm.conf\n"
    "\n"
    "  # Explicit naming via --name N=PATH (overrides the stem default).\n"
    "  f5 query --name pri=tier1.conf --name sec=tier2.conf \\\n"
    "    '$pri.ltm.pool[].name' tier1.conf tier2.conf\n"
    "\n"
    "  # --merge unifies every loaded config into one namespace; .x.y\n"
    "  # iterates across all sources and refs cross files (a GTM pool\n"
    "  # pointing into LTM resolves transparently).  Refuses to merge\n"
    "  # when two sources define the same (kind, full-path).\n"
    "  f5 query --merge '.ltm.pool[] | referenced_by(.)' ltm.conf gtm.conf\n"
    "\n"
    "Run --help-dsl for the grammar, --help-builtins for the function\n"
    "catalogue, and --help-examples for a longer cookbook.\n"
)


class _HelpDslAction(argparse.Action):
    """Print the DSL grammar reference and exit."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        sys.stdout.write(format_grammar())
        parser.exit()


class _HelpBuiltinsAction(argparse.Action):
    """Print the builtin catalogue (optionally narrowed to one name)."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        name = values if isinstance(values, str) and values else None
        sys.stdout.write(format_builtins(name))
        parser.exit()


class _HelpExamplesAction(argparse.Action):
    """Print the cookbook of worked examples."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        sys.stdout.write(format_examples())
        parser.exit()


@verb(
    "query",
    aliases=("q",),
    help="jq-flavoured DSL for inspecting and rewriting BIG-IP configs.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = _DESCRIPTION
    p.epilog = _EPILOG

    p.add_argument(
        "expression",
        nargs="?",
        help=("Query expression.  Use ``-f FILE`` to read a multi-line query from a file instead."),
    )
    p.add_argument(
        "paths",
        nargs="*",
        help=(
            "bigip.conf / SCF files (one or more).  Pass ``-`` to read a single config from stdin."
        ),
    )
    p.add_argument(
        "-f",
        "--from-file",
        metavar="FILE",
        help=(
            "Read the query expression from FILE.  Mutually exclusive "
            "with passing the expression as the first positional argument."
        ),
    )
    p.add_argument(
        "--name",
        action="append",
        default=[],
        metavar="NAME=PATH",
        help=(
            "Bind a config to the DSL variable ``$NAME`` (repeatable).  "
            "Inside the query, ``$gtm.gtm.wideip[]`` reads from the "
            "source given as ``--name gtm=/path/gtm.conf``.  Without "
            "this flag every input is auto-bound under its filename "
            "stem (``ltm.conf`` -> ``$ltm``), so most users won't need "
            "to set it explicitly.  PATH must also appear as one of "
            "the positional inputs.  Names must be valid DSL "
            "identifiers (letters, digits, ``_``, ``-``; not starting "
            "with a digit)."
        ),
    )
    p.add_argument(
        "--partition",
        action="append",
        default=[],
        metavar="PATH=PARTITION",
        help=(
            "Tell the loader which BIG-IP partition a given source file "
            "belongs to.  Short / partition-relative names inside that "
            "file (``ltm pool web_pool {...}`` without a leading "
            "``/Common/``) get qualified with ``/<partition>/`` so the "
            "rest of the system sees canonical full-paths.  Repeatable; "
            "without ``=`` the value applies to every loaded source.  "
            "Defaults to ``Common`` when unset, matching tmsh's "
            "implicit partition."
        ),
    )
    p.add_argument(
        "--merge",
        action="store_true",
        help=(
            "Treat every loaded config as one logical namespace: "
            "``.ltm.virtual[]`` returns virtuals from every input, "
            "and ``refs`` / ``referenced_by`` walk references across "
            "files (a GTM pool pointing into an LTM tier resolves "
            "transparently).  Edits route back to the source they "
            "originated from.  Refuses to merge when two sources "
            "define the same ``(kind, full-path)`` — namespace or "
            "redact the inputs first.  Without --merge the query runs "
            "once per file (back-compat); $name still reaches across "
            "files in either mode."
        ),
    )

    out_group = p.add_mutually_exclusive_group()
    out_group.add_argument(
        "--scf",
        dest="output_mode",
        action="store_const",
        const="scf",
        help=(
            "Render every selected value as an SCF stanza when possible.  "
            "Useful for ``f5 query ... | f5 cleanup``-style pipelines."
        ),
    )
    out_group.add_argument(
        "--raw",
        dest="output_mode",
        action="store_const",
        const="raw",
        help="Render scalar values one per line, no quoting.",
    )
    out_group.add_argument(
        "--paths-only",
        dest="output_mode",
        action="store_const",
        const="paths",
        help="Print only the full-path of each object / reference produced.",
    )
    out_group.add_argument(
        "--json",
        dest="output_mode",
        action="store_const",
        const="json",
        help="Render the result as a JSON array.",
    )

    write_group = p.add_mutually_exclusive_group()
    write_group.add_argument(
        "--write",
        action="store_true",
        help=(
            "When the query mutates, print the rewritten config to stdout "
            "(default: print a unified-diff preview)."
        ),
    )
    write_group.add_argument(
        "--in-place",
        action="store_true",
        help=("When the query mutates, overwrite each input file with the rewritten config."),
    )
    p.add_argument(
        "--strict",
        action="store_true",
        help=(
            "Exit with status 2 (and a stderr message) when a mutating "
            "query produces no textual change — i.e. nothing matched.  "
            "Without --strict, a zero-match mutation returns 1 silently "
            "(tolerant: useful interactively).  With --strict it becomes "
            "a hard error, which is the shape CI / scripted change "
            "pipelines need so a typo in the path doesn't silently "
            "land a no-op build."
        ),
    )

    add_format_arg(p, tmsh_default_verb="modify")

    p.add_argument(
        "--help-dsl",
        nargs=0,
        action=_HelpDslAction,
        default=argparse.SUPPRESS,
        help="Show the DSL grammar reference and exit.",
    )
    p.add_argument(
        "--help-builtins",
        nargs="?",
        const="",
        metavar="NAME",
        action=_HelpBuiltinsAction,
        default=argparse.SUPPRESS,
        help=(
            "Show the catalogue of builtin functions and exit.  Pass a "
            "function name to narrow the output (e.g. "
            "``--help-builtins ip``)."
        ),
    )
    p.add_argument(
        "--help-examples",
        nargs=0,
        action=_HelpExamplesAction,
        default=argparse.SUPPRESS,
        help="Show a cookbook of worked example queries and exit.",
    )

    p.set_defaults(handler=_run_query, output_mode="auto")


def _run_query(args: argparse.Namespace) -> int:
    expression = _resolve_expression(args)
    if expression is None:
        print(
            "error: no query expression supplied (positional or --from-file)",
            file=sys.stderr,
        )
        return 2
    if not args.paths:
        print("error: no input files (pass '-' to read stdin)", file=sys.stderr)
        return 2

    # ``--format tmsh`` re-renders the parsed config as a ``tmsh
    # modify`` script — useful for stdout / pipes / dedicated output
    # files, but never a safe replacement for the on-disk SCF source.
    # Without this guard, ``f5 query --in-place --format tmsh`` would
    # overwrite ``bigip.conf`` with a tmsh script and the dry-run
    # diff (always SCF↔SCF) would still look normal beforehand.
    if args.in_place and args.output_format in ("tmsh", "tmsh-delta"):
        print(
            f"error: --in-place is incompatible with --format {args.output_format} "
            "(in-place writes must preserve the SCF source format; "
            "use --write or redirect to an explicit output file for "
            "tmsh script output)",
            file=sys.stderr,
        )
        return 2

    names_map, name_err = _parse_name_bindings(args.name, args.paths)
    if name_err is not None:
        print(f"error: {name_err}", file=sys.stderr)
        return 2

    partitions_by_path, partition_err = _parse_partition_bindings(args.partition, args.paths)
    if partition_err is not None:
        print(f"error: {partition_err}", file=sys.stderr)
        return 2

    sources: dict[str, str] = {}
    path_for_uri: dict[str, str] = {}
    for path_str in args.paths:
        if path_str == "-" and args.in_place:
            print("error: --in-place requires a path, not stdin", file=sys.stderr)
            return 2
        try:
            # Strict UTF-8 for mutating in-place writes: if any byte
            # in the source can't be decoded, raise instead of
            # silently swapping it for U+FFFD — otherwise the
            # round-trip ``read … rewrite … write_text`` would
            # permanently overwrite the unreadable bytes.
            uri, src = read_path(path_str, strict=args.in_place)
        except (OSError, UnicodeDecodeError) as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2
        if uri in sources:
            # A repeated ``-`` would also silently overwrite — the
            # first read drains stdin, the second produces an empty
            # string.  Reject duplicates so the user gets a clear
            # error rather than a confusing empty-projection result.
            prev = path_for_uri[uri]
            label = "stdin" if path_str == "-" else path_str
            if prev == path_str:
                print(f"error: duplicate input {label}", file=sys.stderr)
            else:
                print(
                    f"error: duplicate input {label} (already read as {prev})",
                    file=sys.stderr,
                )
            return 2
        sources[uri] = src
        path_for_uri[uri] = path_str

    # Translate ``--name N=PATH`` from path-string to URI keys so the
    # runner can look each one up in ``sources``.  Paths that didn't
    # parse / weren't read are flagged before run_query is called so
    # the user gets a precise error rather than a silent miss.
    resolved_names: dict[str, str] = {}
    for nm, path_str in names_map.items():
        # ``path_for_uri`` is keyed URI -> original path; invert it.
        uri_for_path = {p: u for u, p in path_for_uri.items()}
        uri = uri_for_path.get(path_str)
        if uri is None:
            print(
                f"error: --name {nm}={path_str}: path was not loaded "
                "(must also appear as a positional argument)",
                file=sys.stderr,
            )
            return 2
        resolved_names[nm] = uri

    # Translate ``--partition PATH=PARTITION`` keys into URI form so
    # ``run_query``'s contextvar lookup uses the same key shape the
    # source map does.  ``args.partition`` may also include a bare
    # ``--partition NAME`` (applied to every source) which the
    # parser surfaces with ``path_str == ""``.
    resolved_partitions: dict[str, str] = {}
    uri_for_path = {p: u for u, p in path_for_uri.items()}
    for path_str, partition in partitions_by_path.items():
        if path_str == "":
            for uri in sources:
                resolved_partitions.setdefault(uri, partition)
            continue
        uri = uri_for_path.get(path_str)
        if uri is None:
            print(
                f"error: --partition {path_str}={partition}: path was "
                "not loaded (must also appear as a positional argument)",
                file=sys.stderr,
            )
            return 2
        resolved_partitions[uri] = partition

    try:
        result = run_query(
            expression,
            sources,
            names=resolved_names or None,
            merge=args.merge,
            partitions=resolved_partitions or None,
        )
    except QueryError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if result.has_mutation:
        return _emit_mutation(args, sources, result, path_for_uri)
    return _emit_values(args, result, sources)


def _parse_name_bindings(raw: list[str], paths: list[str]) -> tuple[dict[str, str], str | None]:
    """Parse ``--name N=PATH`` entries.

    Returns ``(bindings, error_or_None)``.  Validates that:

    - each entry contains exactly one ``=``;
    - each name is a valid DSL identifier (matches what the lexer
      accepts after ``$``);
    - names are unique;
    - PATH appears in the positional inputs (caller cross-checks).
    """
    import re as _re

    name_re = _re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")
    bindings: dict[str, str] = {}
    paths_set = set(paths)
    for entry in raw:
        if "=" not in entry:
            return {}, f"--name expects NAME=PATH (got {entry!r})"
        nm, _, pth = entry.partition("=")
        if not name_re.match(nm):
            return {}, (
                f"--name {entry!r}: {nm!r} is not a valid DSL identifier "
                "(letters, digits, '_', '-'; cannot start with a digit)"
            )
        if nm in bindings:
            return {}, f"--name {nm}: duplicate binding"
        if pth not in paths_set:
            return {}, (
                f"--name {nm}={pth}: path was not given as a positional "
                "input (so the runner would have no source text for it)"
            )
        bindings[nm] = pth
    return bindings, None


def _parse_partition_bindings(
    raw: list[str], paths: list[str]
) -> tuple[dict[str, str], str | None]:
    """Parse ``--partition PATH=PARTITION`` (or bare ``--partition PARTITION``)
    entries.

    Returns ``(bindings, error_or_None)`` where ``bindings`` maps:

    * an empty string (``""``) to a partition applied to every loaded
      source — the bare ``--partition NAME`` form;
    * each loaded ``PATH`` to its explicit partition name.

    Validates that each ``PATH=`` form names a positional input, that
    a single source isn't bound twice, and that the bare-form is used
    at most once.
    """
    bindings: dict[str, str] = {}
    paths_set = set(paths)
    seen_paths: set[str] = set()
    for entry in raw:
        if "=" in entry:
            pth, _, partition = entry.partition("=")
            if not pth:
                return {}, f"--partition {entry!r}: PATH side cannot be empty"
            if not partition:
                return {}, f"--partition {entry!r}: PARTITION side cannot be empty"
            if pth not in paths_set:
                return {}, (
                    f"--partition {pth}={partition}: path was not given as "
                    "a positional input (so the runner would have no source "
                    "for it)"
                )
            if pth in seen_paths:
                return {}, f"--partition {pth}: duplicate binding"
            seen_paths.add(pth)
            bindings[pth] = partition
        else:
            # Bare ``--partition NAME`` applies to every source.
            if "" in bindings:
                return {}, "--partition: bare PARTITION may only be given once"
            bindings[""] = entry
    return bindings, None


def _resolve_expression(args: argparse.Namespace) -> str | None:
    if args.from_file:
        # When --from-file is set the positional ``expression`` is
        # always the first input file (argparse fills the positional
        # slot before it gets to ``paths``).  Promote it unconditionally
        # so commands like ``f5 query -f q.fq a.conf b.conf`` see both
        # files; the previous ``and not args.paths`` guard silently
        # dropped ``a.conf`` when more than one input was given.
        if args.expression:
            args.paths = [args.expression, *args.paths]
            args.expression = None
        try:
            return Path(args.from_file).read_text(encoding="utf-8")
        except OSError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return None
    return args.expression


def _emit_mutation(
    args: argparse.Namespace,
    sources: dict[str, str],
    result,
    path_for_uri: dict[str, str],
) -> int:
    any_changed = False
    for uri, applied in result.edits_per_file.items():
        for rep in applied.rename_reports:
            print(
                f"renamed {rep.old!r} -> {rep.new!r} ({rep.occurrences} occurrence(s))",
                file=sys.stderr,
            )
        # A "mutating" query that produced no actual textual change
        # (e.g. ``rename_partition(...)`` on a source with no matches)
        # should report no-op via exit code 1, mirroring the
        # convention ``f5 rename`` already uses.
        if applied.new_source == applied.original:
            continue
        any_changed = True
        path_str = path_for_uri.get(uri, uri)
        # ``--format tmsh`` re-renders the rewritten SCF as a
        # ``tmsh modify`` script for persisting onto a live device.
        # The unified-diff path stays SCF↔SCF because the on-disk
        # source is always SCF.
        rewritten = render_config(
            applied.new_source,
            fmt=args.output_format,
            tmsh_verb="modify",
            transaction=getattr(args, "output_transaction", False),
            original=applied.original,
        )
        if args.in_place and path_str != "-":
            Path(path_str).write_text(rewritten, encoding="utf-8")
            continue
        if args.write:
            sys.stdout.write(rewritten)
            continue
        diff = difflib.unified_diff(
            applied.original.splitlines(keepends=True),
            applied.new_source.splitlines(keepends=True),
            fromfile=path_str,
            tofile=f"{path_str} (modified)",
        )
        sys.stdout.writelines(diff)
    if not any_changed:
        if getattr(args, "strict", False):
            print(
                "error: --strict: mutating query produced no textual "
                "change (no matches).  Check the path / predicate.",
                file=sys.stderr,
            )
            return 2
        return 1
    return 0


def _emit_values(
    args: argparse.Namespace,
    result,
    sources: dict[str, str],
) -> int:
    any_matched = False
    multi = len(sources) > 1
    # Per-file ``# === uri ===`` banners are line-oriented; emitting
    # them around ``--json`` output corrupts the JSON document.  Skip
    # them for json — callers wanting per-file grouping can run the
    # query once per file (``for f in *.conf; do f5 query -j ... $f``).
    use_banner = multi and args.output_mode != "json"

    # Multi-file ``--json`` must produce a single top-level JSON
    # document, not a stream of adjacent arrays — otherwise tools
    # like ``jq``, ``python -m json.tool``, and any ``json.load``
    # consumer reject the output as invalid JSON.  Emit one
    # ``[{"uri": ..., "values": [...]}, ...]`` envelope per
    # invocation; single-file invocations stay flat (just the values
    # array) so the simple case keeps its expected shape.
    if multi and args.output_mode == "json":
        envelope: list[dict] = []
        for uri, values in result.values_per_file.items():
            if values:
                any_matched = True
            envelope.append(
                {
                    "uri": uri,
                    "values": json.loads(render(values, mode="json")),
                }
            )
        sys.stdout.write(json.dumps(envelope, indent=2) + "\n")
        return _empty_match_exit_code(args, any_matched)

    for uri, values in result.values_per_file.items():
        # "Matched" means the evaluator produced at least one value
        # for this source — empty strings, ``null``, ``false``, and
        # zero-length renders all count as matches.  Earlier this
        # branch keyed off the rendered text being truthy, which
        # incorrectly reported "no results" when a query landed on
        # an empty ``full-path`` or an empty paths-only list.
        if values:
            any_matched = True
        if use_banner:
            sys.stdout.write(f"# === {uri} ===\n")
        sys.stdout.write(render(values, mode=args.output_mode))
    return _empty_match_exit_code(args, any_matched)


def _empty_match_exit_code(args: argparse.Namespace, any_matched: bool) -> int:
    """Decide the read-only query exit code.

    Default mirrors jq: ``0`` whether or not the query produced
    matches — successful evaluation is its own success signal.
    ``--strict`` opts in to jq's ``-e``-style "exit 1 when nothing
    matched" semantics, which is what scripted pipelines that treat
    a zero-result run as an error want.
    """
    if any_matched:
        return 0
    if getattr(args, "strict", False):
        return 1
    return 0
