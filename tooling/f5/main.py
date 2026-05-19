"""Command-line entry point for the ``f5`` CLI.

This is a separate top-level CLI from :mod:`explorer.tcl_cli`.  Its
verb registry lives in :mod:`explorer.verbs.f5`, which is decoupled
from the ``tcl`` / ``irule`` registry under :mod:`explorer.verbs`.

Run as::

    python -m explorer.f5_cli cleanup samples/bigip/bigip.conf

Or, once the ``f5`` console-script entry is wired up by the zipapp
build, simply::

    f5 cleanup samples/bigip/bigip.conf
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    from server._build_info import BUILD_TIMESTAMP, FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"
    BUILD_TIMESTAMP = ""

from tooling.explorer.verbs.f5 import load_verbs
from tooling.explorer.verbs.f5._registry import apply_verb_registrations, get_verb_catalogue
from tooling.explorer.verbs.f5.irule import add_irule_subparser


def _version_string() -> str:
    version = FULL_VERSION
    if BUILD_TIMESTAMP:
        version += f" ({BUILD_TIMESTAMP})"
    return version


def _infer_prog_name(argv0: str) -> str:
    raw_name = Path(argv0).name.strip()
    if not raw_name:
        return "f5"
    stem = Path(raw_name).stem.lower()
    if not stem or stem.startswith("python"):
        return "f5"
    # Strip the ``f5-<version>`` suffix the zipapp builder bakes into
    # the filename so the brief-help output reads ``f5 <verb> ...``.
    if stem.startswith("f5-"):
        return "f5"
    return stem


_GROUP_VERBS: tuple[tuple[str, str], ...] = (
    ("irule", "iRules-specific analysis (event-order, event-info, ...)."),
)


class _BriefHelpAction(argparse.Action):
    """Print a compact help overview and exit."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        prog = parser.prog
        lines = [
            f"{prog} <verb> [options] [inputs...]\n",
            "Verbs:",
        ]
        for name, alias, desc in get_verb_catalogue():
            alias_str = f" ({alias})" if alias else ""
            label = f"{name}{alias_str}"
            lines.append(f"  {label:<22s} {desc}")
        for name, desc in _GROUP_VERBS:
            lines.append(f"  {name:<22s} {desc}")
        lines.append("")
        lines.append("Options:")
        lines.append("  -h, --help              Show this help and exit.")
        lines.append("  --help-all              Show full help for every verb.")
        lines.append("  --version               Print the build version and exit.")
        parser.exit(message="\n".join(lines) + "\n")


class _FullHelpAction(argparse.Action):
    """Print full help for every registered verb and exit."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        parser.print_help()
        sub_actions = [
            action for action in parser._actions if isinstance(action, argparse._SubParsersAction)
        ]
        for sub_action in sub_actions:
            for verb_name, verb_parser in sub_action.choices.items():
                print(f"\n=== {verb_name} ===\n")
                verb_parser.print_help()
        parser.exit()


def _build_parser(prog_name: str) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=prog_name,
        description=(
            "BIG-IP configuration analysis CLI.  Operates on bigip.conf / "
            "SCF files; verbs are registered separately from the ``tcl`` and "
            "``irule`` CLIs and may be used independently."
        ),
        add_help=False,
    )
    parser.add_argument(
        "-h",
        "--help",
        nargs=0,
        action=_BriefHelpAction,
        default=argparse.SUPPRESS,
        help="Show brief help and exit.",
    )
    parser.add_argument(
        "--help-all",
        nargs=0,
        action=_FullHelpAction,
        default=argparse.SUPPRESS,
        help="Show full help for every verb and exit.",
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"{prog_name} {_version_string()}",
    )

    sub = parser.add_subparsers(dest="verb", required=True)

    load_verbs()
    apply_verb_registrations(sub, prog_name=prog_name, default_dialect="f5-bigip")
    add_irule_subparser(sub, prog_name=prog_name, default_dialect="f5-irules")
    return parser


def build_parser(prog_name: str = "f5") -> argparse.ArgumentParser:
    """Build the top-level argparse parser without consuming argv.

    Exposed so that :mod:`argcomplete` (and the ``completion`` verb that
    drives it) can introspect the verb tree without invoking ``parse_args``.
    """
    return _build_parser(prog_name)


def main(argv: list[str] | None = None) -> int:
    if argv is None:
        parsed_argv = sys.argv[1:]
        prog_name = _infer_prog_name(sys.argv[0])
    else:
        parsed_argv = argv
        prog_name = "f5"

    parser = _build_parser(prog_name)
    from tooling.explorer._argcomplete_support import autocomplete

    autocomplete(parser)
    args = parser.parse_args(parsed_argv)

    try:
        handler = args.handler
    except AttributeError as exc:  # pragma: no cover - argparse enforces this.
        raise RuntimeError("internal error: no command handler selected") from exc

    try:
        return handler(args)
    except Exception as exc:  # noqa: BLE001
        print(f"error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
