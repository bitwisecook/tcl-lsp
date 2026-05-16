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
  same grammar reference that lives in ``docs/references/f5_query/dsl.md``.
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


class _HelpManualAction(argparse.Action):
    """Print the comprehensive manual — grammar + builtins + examples.

    Gives MCP / AI surfaces a single self-contained reference they
    can feed back to a model when answering questions about the
    DSL.  Identical content to ``--help-dsl`` + ``--help-builtins``
    + ``--help-examples`` concatenated, with section banners that
    make the output easy to scan or chunk for context windows.
    """

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        bar = "=" * 72
        parts = (
            f"{bar}\nGRAMMAR\n{bar}\n\n",
            format_grammar(),
            f"\n\n{bar}\nBUILTINS\n{bar}\n\n",
            format_builtins(None),
            f"\n\n{bar}\nEXAMPLES\n{bar}\n\n",
            format_examples(),
        )
        for part in parts:
            sys.stdout.write(part)
        parser.exit()


class _HelpInputFormatsAction(argparse.Action):
    """Print the catalogue of registered input formats (built-in + plugin).

    Mirrors ``--help-renderers`` for the input-format registry: lists
    every format the runner will accept after the user-plugin XDG
    scan has run, so a user can confirm a freshly-dropped
    ``@input_format`` plugin is actually being picked up.
    """

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        from core.bigip.query import list_input_formats

        specs = list_input_formats()
        if not specs:
            sys.stdout.write("(no input formats registered)\n")
            parser.exit()
        sys.stdout.write("Registered input formats:\n\n")
        for spec in specs:
            sys.stdout.write(f"  {spec.name}\n")
            sys.stdout.write(f"    summary: {spec.summary}\n")
            if spec.details:
                for line in spec.details.splitlines():
                    sys.stdout.write(f"    {line}\n")
            sys.stdout.write("\n")
        sys.stdout.write("Use --input KIND NAME=PATH to bind a file via any registered format.\n")
        parser.exit()


class _HelpPluginsAction(argparse.Action):
    """Print the XDG plugin directory and the files the loader picked up.

    Diagnostic output for "is my plugin file being loaded?" — prints
    the directory path the loader scans, then runs the loader and
    prints every file that imported successfully.  Plugin import
    failures are written to stderr by the loader itself, so combining
    this with ``2>&1`` shows successes + failures together.
    """

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        from core.bigip.query import load_user_plugins, xdg_plugin_dir

        directory = xdg_plugin_dir()
        sys.stdout.write(f"Plugin directory: {directory}\n")
        if not directory.is_dir():
            sys.stdout.write("  (does not exist — create it to start dropping plugins)\n")
            parser.exit()
        loaded = load_user_plugins(force=True)
        if not loaded:
            sys.stdout.write("  (no plugin files found)\n")
        else:
            sys.stdout.write("Loaded plugin files:\n")
            for path in loaded:
                sys.stdout.write(f"  {path}\n")
        parser.exit()


class _HelpRenderersAction(argparse.Action):
    """Print the catalogue of registered renderer plugins.

    Mirrors ``--help-builtins`` for the renderer registry: imports
    every built-in renderer (mermaid / gantt / ascii-blocks) so they
    self-register, then prints each spec's name, summary, and
    accepted input shape so a user can pick the right ``--render
    NAME`` from one terminal command.
    """

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        from core.bigip.query import list_renderers

        specs = list_renderers()
        if not specs:
            sys.stdout.write("(no renderers registered)\n")
            parser.exit()
        sys.stdout.write("Registered renderers:\n\n")
        for spec in specs:
            sys.stdout.write(f"  {spec.name}\n")
            sys.stdout.write(f"    summary: {spec.summary}\n")
            sys.stdout.write(f"    accepts: {spec.accepts}\n")
            if spec.details:
                # Indent details under the spec; soft-wrap is the
                # renderer author's responsibility.
                for line in spec.details.splitlines():
                    sys.stdout.write(f"    {line}\n")
            sys.stdout.write("\n")
        sys.stdout.write(
            "Use --render NAME to dispatch, --render-opt KEY=VALUE for per-renderer options.\n"
        )
        parser.exit()


class _HelpReferencesAction(argparse.Action):
    """Print the comprehensive `f5 query` reference manual.

    Sources ``docs/references/f5_query/manual.md`` — the master
    long-form companion to the auto-generated grammar / builtins
    / examples surfaces.  Covers stream semantics, the probe gate
    + reason taxonomy, cert-dict shape, mutating-query apply
    order, the F5 KB cross-reference, sample SCF fragments, cert-
    generation one-liners, and end-to-end audit walkthroughs.

    Each section carries a stable Markdown anchor so external
    tools (MCP, AI skills, IDE quick-lookups) can deep-link into
    a single concept.  Pair with ``--help-builtins NAME`` for the
    full per-function documentation.
    """

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        from pathlib import Path

        # The manual lives in the repo's ``docs/references`` tree.
        # Resolve relative to this verb module so editable installs
        # and zipapp builds both find it.
        repo_root = Path(__file__).resolve().parents[3]
        manual = repo_root / "docs" / "references" / "f5_query" / "manual.md"
        if manual.is_file():
            sys.stdout.write(manual.read_text(encoding="utf-8"))
            parser.exit()
        # Fall back to the F5 KB cross-reference doc if only it is
        # bundled — better than nothing, and the manual links to
        # it explicitly.
        kb = repo_root / "docs" / "references" / "f5_query" / "f5-kb-monitor-articles.md"
        if kb.is_file():
            sys.stdout.write(kb.read_text(encoding="utf-8"))
            parser.exit()
        sys.stdout.write(
            "f5 query --help-references: references not found "
            f"under {repo_root / 'docs' / 'references'}\n"
            "(this build may not include the docs/ tree; see "
            "https://github.com/bitwisecook/tcl-lsp for the canonical copy)\n"
        )
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
        "--enable-probes",
        action="store_true",
        help=(
            "Opt the query in to live network probes "
            "(``ping``, ``portping``, ``traceroute``, ``url_get`` "
            "and friends, ``socket_get``, ``tls_handshake``).  "
            "Without this flag those builtins raise rather than "
            "touching the network — keeps the default invocation "
            "offline-safe."
        ),
    )
    p.add_argument(
        "--ca-bundle",
        metavar="PATH",
        help=(
            "CA bundle to trust for TLS-aware probes such as "
            "``url_get`` and ``tls_handshake``.  Defaults to the "
            "platform trust store.  Only used when a query runs a "
            "TLS probe."
        ),
    )
    p.add_argument(
        "--input-json",
        action="append",
        default=[],
        metavar="NAME=PATH",
        help=(
            "Bind an external JSON file to ``$NAME`` as a side input "
            "(repeatable).  Unlike positional inputs the JSON file "
            "is not treated as a BIG-IP configuration; the parsed "
            "value (a dict / list / scalar) is the queryable shape "
            "of ``$NAME``.  Use this to mix external data (CMDB, "
            "vlan-to-tenant map, signed-cert manifest) into a "
            "query without reaching for a sidecar tool.  Inside "
            "the DSL you can also use the ``json_load(path)`` "
            "builtin to read JSON ad-hoc."
        ),
    )
    p.add_argument(
        "--input-jsonl",
        action="append",
        default=[],
        metavar="NAME=PATH",
        help=(
            "Bind a JSON Lines (NDJSON) file to ``$NAME`` as a side "
            "input (repeatable).  Each non-blank line of the file "
            "is parsed as one JSON value and ``$NAME`` resolves to "
            "the resulting list, so a query can iterate logs / "
            "event archives the same way it iterates a BIG-IP "
            'collection: ``$NAME[] | select(.severity == "err")``.  '
            "Inside the DSL the ``jsonl_load(path)`` builtin gives "
            "the same shape ad-hoc."
        ),
    )
    p.add_argument(
        "--input-csv",
        action="append",
        default=[],
        metavar="NAME=PATH[:hdr1,hdr2,…]",
        help=(
            "Bind a CSV file to ``$NAME`` as a side input "
            "(repeatable).  Without the trailing ``:hdr1,hdr2,…`` "
            "the first row of the file names the columns (the "
            "common spreadsheet shape).  With the trailing "
            "header list every row is data and the supplied "
            "names label each column — use this form for "
            "header-less CSVs (firewall NAT exports, RFC 4180 "
            "fragments).  Values are strings; the DSL's "
            "``+`` operator coerces them on demand.  Inside the "
            "DSL the ``csv_load(path[, headers])`` builtin "
            "gives the same shape ad-hoc."
        ),
    )
    p.add_argument(
        "--input-f5log",
        action="append",
        default=[],
        metavar="NAME=PATH",
        help=(
            "Bind a BIG-IP log file (``/var/log/ltm``, "
            "``/var/log/tmm``, audit, hsl) to ``$NAME`` as a "
            "side input (repeatable).  Each line is parsed into "
            "a structured event dict (``{timestamp, host, "
            "severity, daemon, pid, code, module, level, "
            "message, raw}``) so a query can filter by F5 "
            "message code or severity without sub-parsing the "
            'line: ``$NAME[] | select(.module == "01070417")``.  '
            "Inside the DSL the ``f5log_load(path)`` builtin "
            "gives the same shape ad-hoc."
        ),
    )
    p.add_argument(
        "--input",
        action="append",
        default=[],
        nargs=2,
        metavar=("KIND", "NAME=PATH"),
        help=(
            "Bind a file via a registered input format (repeatable).  "
            "KIND is the format name (built-in: ``json``, ``jsonl``, "
            "``csv``, ``f5log``; user plugins add more via "
            "``@input_format`` in ``$XDG_CONFIG_HOME/f5q/plugins/``).  "
            "Equivalent to the typed ``--input-<kind>`` flags but "
            "extensible — use this for user-defined formats like "
            "``--input yaml routes=routes.yaml``."
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
    out_group.add_argument(
        "--table",
        dest="output_mode",
        action="store_const",
        const="table",
        help=(
            "Render the result as an ASCII grid.  Column headers are "
            "discovered from the first object-literal row's keys; each "
            "subsequent row lays out under the same columns.  Use "
            "``--table-lineart`` for the Unicode box-drawing variant."
        ),
    )
    out_group.add_argument(
        "--table-lineart",
        dest="output_mode",
        action="store_const",
        const="table-lineart",
        help=(
            "Like ``--table`` but uses Unicode box-drawing characters "
            "(``┌─┬─┐`` / ``│``) for the borders — prettier in modern "
            "terminals."
        ),
    )
    out_group.add_argument(
        "-R",
        "--render",
        dest="render_name",
        metavar="NAME",
        default=None,
        help=(
            "Dispatch the result to the named renderer plugin "
            "(``mermaid`` / ``gantt`` / ``ascii-blocks`` built-in, "
            "plus any user-registered renderers).  See "
            "``--help-renderers`` for the catalogue and "
            "``--render-opt KEY=VALUE`` for per-renderer options."
        ),
    )
    p.add_argument(
        "--render-opt",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        help=(
            "Pass an option to ``--render NAME`` (repeatable).  Keys "
            "and values are renderer-specific — e.g. ``--render gantt "
            "--render-opt unit-minutes=10`` or ``--render mermaid "
            "--render-opt direction=TB``."
        ),
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
    p.add_argument(
        "--help-manual",
        nargs=0,
        action=_HelpManualAction,
        default=argparse.SUPPRESS,
        help=(
            "Show the comprehensive manual (grammar + builtins + examples) "
            "as one self-contained reference and exit.  Useful when feeding "
            "the DSL surface to an AI agent / MCP context."
        ),
    )
    p.add_argument(
        "--help-references",
        nargs=0,
        action=_HelpReferencesAction,
        default=argparse.SUPPRESS,
        help=(
            "Show the embedded F5 KB references (K2167 / K3451 / K3224 / "
            "K12531 etc.) and exit.  Use alongside ``--help-builtins`` when "
            "figuring out which probe builtin matches a device behaviour."
        ),
    )
    p.add_argument(
        "--help-renderers",
        nargs=0,
        action=_HelpRenderersAction,
        default=argparse.SUPPRESS,
        help=(
            "Show the catalogue of registered renderer plugins "
            "(``mermaid``, ``gantt``, ``ascii-blocks`` built-in) and exit."
        ),
    )
    p.add_argument(
        "--help-inputs",
        nargs=0,
        action=_HelpInputFormatsAction,
        default=argparse.SUPPRESS,
        help=(
            "Show the catalogue of registered input formats "
            "(``json``, ``jsonl``, ``csv``, ``f5log`` built-in, plus "
            "any ``@input_format`` plugins) and exit."
        ),
    )
    p.add_argument(
        "--help-plugins",
        nargs=0,
        action=_HelpPluginsAction,
        default=argparse.SUPPRESS,
        help=(
            "Print the XDG plugin directory and every plugin file the "
            "loader picked up, then exit.  Diagnostic for 'is my plugin "
            "actually being loaded?'."
        ),
    )

    p.set_defaults(handler=_run_query, output_mode="auto", render_name=None, input=[])


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

    json_name_to_path, json_err = _parse_json_bindings(args.input_json)
    if json_err is not None:
        print(f"error: {json_err}", file=sys.stderr)
        return 2

    jsonl_bindings, jsonl_err = _parse_input_bindings(args.input_jsonl, flag="--input-jsonl")
    if jsonl_err is not None:
        print(f"error: {jsonl_err}", file=sys.stderr)
        return 2

    csv_bindings, csv_err = _parse_input_bindings(
        args.input_csv, flag="--input-csv", allow_csv_headers=True
    )
    if csv_err is not None:
        print(f"error: {csv_err}", file=sys.stderr)
        return 2

    f5log_bindings, f5log_err = _parse_input_bindings(args.input_f5log, flag="--input-f5log")
    if f5log_err is not None:
        print(f"error: {f5log_err}", file=sys.stderr)
        return 2

    custom_input_bindings, custom_input_err = _parse_custom_input_bindings(args.input)
    if custom_input_err is not None:
        print(f"error: {custom_input_err}", file=sys.stderr)
        return 2

    sources: dict[str, str] = {}
    path_for_uri: dict[str, str] = {}
    for path_str in args.paths:
        if path_str == "-" and args.in_place:
            print("error: --in-place requires a path, not stdin", file=sys.stderr)
            return 2
        if args.in_place and path_str != "-" and Path(path_str).suffix.lower() == ".ucs":
            # UCS is a gzipped tar; rewriting it in place would mean
            # repacking the archive and losing other config artefacts.
            # Refuse rather than silently dropping content.
            print(
                f"error: --in-place not supported for UCS archives ({path_str}); "
                "extract first with `f5 extract` or use --write",
                file=sys.stderr,
            )
            return 2
        try:
            # Strict UTF-8 for mutating in-place writes: if any byte
            # in the source can't be decoded, raise instead of
            # silently swapping it for U+FFFD — otherwise the
            # round-trip ``read … rewrite … write_text`` would
            # permanently overwrite the unreadable bytes.
            uri, src = read_path(path_str, strict=args.in_place)
        except (OSError, UnicodeDecodeError, ValueError) as exc:
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

    # Load every structured side-input (JSON / JSONL / CSV / F5
    # logs) into ``sources`` with a per-URI :class:`InputSpec` so
    # the runner knows how to parse each one.  Names are merged
    # into ``side_resolved_names`` for the ``$NAME`` lookup map.
    # URIs collide-check against positional inputs so a typo
    # doesn't silently overwrite a config.
    from core.bigip.query._inputs import InputSpec as _InputSpec

    input_specs: dict[str, _InputSpec] = {}
    side_resolved_names: dict[str, str] = {}

    def _load_side_input(
        nm: str,
        pth: str,
        flag: str,
        spec: _InputSpec,
    ) -> str | None:
        """Read *pth*, register it under *nm* with *spec*; returns
        an error string or ``None`` on success."""
        try:
            uri, src = read_path(pth, strict=False)
        except (OSError, UnicodeDecodeError) as exc:
            return f"{flag} {nm}={pth}: {exc}"
        if uri in sources:
            return f"{flag} {nm}={pth}: URI {uri} is already loaded as a positional input"
        if uri in input_specs:
            return f"{flag} {nm}={pth}: URI {uri} is already loaded as a side input"
        sources[uri] = src
        input_specs[uri] = spec
        side_resolved_names[nm] = uri
        return None

    for nm, json_path in json_name_to_path.items():
        err = _load_side_input(nm, json_path, "--input-json", _InputSpec(kind="json"))
        if err is not None:
            print(f"error: {err}", file=sys.stderr)
            return 2

    for nm, (jsonl_path, _hdr) in jsonl_bindings.items():
        err = _load_side_input(nm, jsonl_path, "--input-jsonl", _InputSpec(kind="jsonl"))
        if err is not None:
            print(f"error: {err}", file=sys.stderr)
            return 2

    for nm, (csv_path, csv_hdr) in csv_bindings.items():
        options: tuple[tuple[str, object], ...] = (("headers", tuple(csv_hdr)),) if csv_hdr else ()
        err = _load_side_input(nm, csv_path, "--input-csv", _InputSpec(kind="csv", options=options))
        if err is not None:
            print(f"error: {err}", file=sys.stderr)
            return 2

    for nm, (f5log_path, _hdr) in f5log_bindings.items():
        err = _load_side_input(nm, f5log_path, "--input-f5log", _InputSpec(kind="f5log"))
        if err is not None:
            print(f"error: {err}", file=sys.stderr)
            return 2

    # Generic ``--input KIND NAME=PATH`` form — covers user plugins and
    # any future built-in formats without needing a dedicated flag.
    # Format validity is checked against the registered input formats
    # so a typo in KIND surfaces as a clear error before file IO.
    from core.bigip.query.inputs import list_input_formats as _list_input_formats
    from core.bigip.query.inputs import lookup as _lookup_input

    for kind, nm, custom_path in custom_input_bindings:
        if _lookup_input(kind) is None:
            registered = ", ".join(s.name for s in _list_input_formats())
            print(
                f"error: --input {kind} {nm}={custom_path}: unknown input "
                f"format {kind!r} (registered: {registered})",
                file=sys.stderr,
            )
            return 2
        err = _load_side_input(nm, custom_path, f"--input {kind}", _InputSpec(kind=kind))
        if err is not None:
            print(f"error: {err}", file=sys.stderr)
            return 2

    # Back-compat: the runner used to take a frozenset of JSON-source
    # URIs.  ``input_specs`` is the strictly-more-general successor;
    # keep ``json_source_uris`` available for the legacy ``--json``
    # path-only entries because nothing downstream relies on it yet.
    json_resolved_names = side_resolved_names

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

    # Merge the JSON-side names into the resolved-names map so
    # ``$NAME`` from ``--json NAME=PATH`` resolves the same way
    # ``$NAME`` from ``--name NAME=PATH`` does.
    full_names = {**resolved_names, **json_resolved_names}

    # Network-probe builtins (ping, portping, url_*, etc.) gate
    # themselves on the ``PROBES_ENABLED`` contextvar.  Set it for
    # the duration of the run_query call so the default invocation
    # stays offline-safe.
    from core.bigip.query._probes import PROBES_ENABLED, TLS_CA_BUNDLE

    _probe_token = PROBES_ENABLED.set(bool(args.enable_probes))
    _ca_bundle_token = TLS_CA_BUNDLE.set(args.ca_bundle)
    try:
        result = run_query(
            expression,
            sources,
            names=full_names or None,
            merge=args.merge,
            partitions=resolved_partitions or None,
            input_specs=input_specs or None,
        )
    except QueryError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    finally:
        PROBES_ENABLED.reset(_probe_token)
        TLS_CA_BUNDLE.reset(_ca_bundle_token)

    # ``--render NAME`` re-uses the same output dispatch path as the
    # built-in modes: ``output.render`` falls through to the renderer
    # registry on an unknown mode, so we just swap ``args.output_mode``
    # for the requested name and let ``_emit_values`` drive the rest.
    if args.render_name is not None:
        args.output_mode = args.render_name
        render_opts, render_err = _parse_render_opts(args.render_opt)
        if render_err is not None:
            print(f"error: {render_err}", file=sys.stderr)
            return 2
        args.render_opts = render_opts
    else:
        args.render_opts = {}

    if result.has_mutation:
        return _emit_mutation(args, sources, result, path_for_uri)
    return _emit_values(args, result, sources)


def _parse_render_opts(raw: list[str]) -> tuple[dict[str, str], str | None]:
    """Parse repeated ``--render-opt KEY=VALUE`` into a flat dict.

    Duplicate keys take the last value (matches argparse's natural
    last-wins semantics on repeated flags).  Returns an error string
    when any entry is malformed; the runner prints it with the
    standard ``error:`` prefix.
    """
    opts: dict[str, str] = {}
    for entry in raw:
        if "=" not in entry:
            return {}, f"--render-opt expects KEY=VALUE (got {entry!r})"
        k, _, v = entry.partition("=")
        if not k:
            return {}, f"--render-opt {entry!r}: KEY side cannot be empty"
        opts[k] = v
    return opts, None


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


def _parse_input_bindings(
    raw: list[str],
    *,
    flag: str,
    allow_csv_headers: bool = False,
) -> tuple[dict[str, tuple[str, list[str] | None]], str | None]:
    """Parse a ``--input-<kind> NAME=PATH[:hdr,…]`` flag group.

    Returns ``(bindings, error)``.  ``bindings`` maps each ``$NAME``
    to a tuple ``(path, csv_headers_or_None)``; the headers list is
    only populated when *allow_csv_headers* is set and the entry has
    a trailing ``:hdr1,hdr2,…`` segment.  All formats share the
    NAME=PATH skeleton so one parser drives JSON / JSONL / CSV /
    f5log entries — the CSV form adds the optional trailing header
    list.
    """
    import re as _re

    name_re = _re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")
    bindings: dict[str, tuple[str, list[str] | None]] = {}
    for entry in raw:
        if "=" not in entry:
            return {}, f"{flag} expects NAME=PATH (got {entry!r})"
        nm, _, pth = entry.partition("=")
        if not name_re.match(nm):
            return {}, (
                f"{flag} {entry!r}: {nm!r} is not a valid DSL identifier "
                "(letters, digits, '_', '-'; cannot start with a digit)"
            )
        if not pth:
            return {}, f"{flag} {entry!r}: PATH side cannot be empty"
        if nm in bindings:
            return {}, f"{flag} {nm}: duplicate binding"
        headers: list[str] | None = None
        if allow_csv_headers:
            pth, headers, err = _split_csv_path_headers(pth, flag=flag, entry=entry)
            if err is not None:
                return {}, err
        bindings[nm] = (pth, headers)
    return bindings, None


def _split_csv_path_headers(
    value: str,
    *,
    flag: str,
    entry: str,
) -> tuple[str, list[str] | None, str | None]:
    """Split ``PATH[:hdr1,hdr2]`` without treating path colons as headers.

    The optional header suffix is only recognised when the final
    colon segment looks like a comma-separated list of simple field
    names.  That keeps POSIX paths containing ``:`` and Windows
    drive paths intact, while preserving the documented compact CSV
    form for header-less files.
    """
    if ":" not in value:
        return value, None, None
    path_part, _, headers_part = value.rpartition(":")
    if not headers_part:
        return (
            value,
            None,
            f"{flag} {entry!r}: trailing ``:`` requires one or more header names (hdr1,hdr2,…)",
        )
    split_headers = [h.strip() for h in headers_part.split(",") if h.strip()]
    if not split_headers:
        return (
            value,
            None,
            f"{flag} {entry!r}: trailing ``:`` requires one or more header names (hdr1,hdr2,…)",
        )
    import re as _re

    header_re = _re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")
    if not all(header_re.match(header) for header in split_headers):
        return value, None, None
    if not path_part:
        return "", None, f"{flag} {entry!r}: PATH side cannot be empty"
    return path_part, split_headers, None


def _parse_custom_input_bindings(
    raw: list[list[str]],
) -> tuple[list[tuple[str, str, str]], str | None]:
    """Parse repeated ``--input KIND NAME=PATH`` argument pairs.

    Returns ``(entries, error_or_None)`` where each entry is a tuple
    ``(kind, name, path)``.  Validates that:

    - ``NAME=PATH`` contains a ``=``;
    - ``NAME`` is a valid DSL identifier;
    - ``KIND`` and ``NAME`` are both non-empty;
    - no two entries bind the same ``NAME``.

    ``KIND`` is not validated against the registry here — the caller
    does that lookup after parsing so it can produce a friendlier
    "registered: ..." error listing every format the run knows about.
    """
    import re as _re

    name_re = _re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")
    entries: list[tuple[str, str, str]] = []
    seen_names: set[str] = set()
    for pair in raw:
        if len(pair) != 2:
            return [], (f"--input expects two arguments KIND NAME=PATH (got {pair!r})")
        kind, binding = pair
        if not kind:
            return [], f"--input {pair!r}: KIND cannot be empty"
        if "=" not in binding:
            return [], f"--input {kind} expects NAME=PATH (got {binding!r})"
        nm, _, pth = binding.partition("=")
        if not name_re.match(nm):
            return [], (
                f"--input {kind} {binding!r}: {nm!r} is not a valid DSL "
                "identifier (letters, digits, '_', '-'; cannot start with a digit)"
            )
        if not pth:
            return [], f"--input {kind} {binding!r}: PATH side cannot be empty"
        if nm in seen_names:
            return [], f"--input {kind} {nm}: duplicate binding"
        seen_names.add(nm)
        entries.append((kind, nm, pth))
    return entries, None


def _parse_json_bindings(raw: list[str]) -> tuple[dict[str, str], str | None]:
    """Back-compat wrapper for ``--input-json``: returns the legacy
    ``{name: path}`` shape so older call sites don't have to know
    about the headers tuple form.
    """
    raw_bindings, err = _parse_input_bindings(raw, flag="--input-json")
    if err is not None:
        return {}, err
    return {nm: pth for nm, (pth, _) in raw_bindings.items()}, None


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

    render_opts = getattr(args, "render_opts", {}) or {}
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
        try:
            sys.stdout.write(render(values, mode=args.output_mode, **render_opts))
        except QueryError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2
        except ValueError as exc:
            # Surface unknown-mode errors as ``error:`` rather than a
            # traceback — ``output.render`` raises ValueError for an
            # unregistered renderer name, matching the historical
            # contract for the built-in modes.
            print(f"error: {exc}", file=sys.stderr)
            return 2
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
