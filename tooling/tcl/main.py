"""Unified verb-based Tcl CLI.

This module powers the ``tcl`` zipapp and exposes task-focused verbs:

- ``opt``: optimise source text and emit rewritten Tcl.
- ``diag``: run diagnostics across one or more inputs.
- ``lint``: run diagnostics/lint checks across one or more inputs.
- ``validate``: run validation (error-level diagnostics only).
- ``format``: reformat source text with canonical style rules.
- ``minify``: minify source by stripping comments, collapsing whitespace, joining commands.
- ``symbols``: emit symbol definitions for the resolved source.
- ``diagram``: extract control-flow diagram data from compiler IR.
- ``callgraph``: build procedure call graph data.
- ``symbolgraph``: build symbol relationship graph data.
- ``dataflow``: build taint/effect data-flow graph data.
- ``event-order``: show iRules events in canonical firing order.
- ``event-info``: look up iRules event metadata and valid commands.
- ``command-info``: look up command registry metadata.
- ``find-legacy``: detect legacy patterns eligible for modernisation.
- ``dis``: disassemble compiled bytecode.
- ``compwasm``: compile to WebAssembly binary output.
- ``highlight``: emit syntax-highlighted source (ANSI or HTML).
- ``diff``: compare two Tcl/iRules sources via AST, IR, and CFG layers.
- ``explore``: run compiler-explorer views on aggregated input.
- ``help``: search bundled KCS help docs from the SQLite index.
"""

from __future__ import annotations

import argparse
import configparser
import os
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

try:
    from shared._build_info import BUILD_TIMESTAMP, FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"
    BUILD_TIMESTAMP = ""

from shared.codes import (
    default_disabled_diagnostics as _default_disabled_diagnostics,
)
from shared.codes import (
    diagnostic_codes as _diagnostic_codes,
)
from tooling.explorer.verbs._registry import apply_verb_registrations, get_verb_catalogue
from tooling.explorer.verbs._utils import TclCliError
from tooling.explorer.verbs.lookup import (
    _load_help_queries as _load_help_queries,  # re-exported; tests monkeypatch this attribute
)
from tooling.formatter.config import FormatterConfig

_ALL_DIAGNOSTIC_CODES = _diagnostic_codes()

_ALL_OPTIMISATION_CODES = frozenset(
    {
        "O100",
        "O101",
        "O102",
        "O103",
        "O104",
        "O105",
        "O106",
        "O107",
        "O108",
        "O109",
        "O110",
        "O111",
        "O112",
        "O113",
        "O114",
        "O115",
        "O116",
        "O117",
        "O118",
        "O119",
        "O120",
        "O121",
        "O122",
        "O123",
        "O124",
        "O125",
        "O126",
    }
)


@dataclass
class _CliConfig:
    """Resolved CLI configuration (INI + defaults)."""

    colour_mode: str = "auto"  # auto | always | never
    tab_width: int = 4
    formatter: FormatterConfig = field(default_factory=FormatterConfig)
    diagnostics_enabled: bool = True
    disabled_diagnostics: set[str] = field(default_factory=set)
    optimiser_enabled: bool = True
    optimiser_profile: str = "full"  # CLI default is full (explicit action)
    disabled_optimisations: set[str] = field(default_factory=set)


def _config_file_paths() -> list[Path]:
    """Return INI config paths in read-order (global first, local last).

    The global config follows platform-native conventions (same logic as
    ``shared.user_config._config_dir``):

    - Linux / BSD / WSL2: ``~/.config/tcl.ini``
    - macOS: ``~/Library/Application Support/tcl.ini``
    - Windows (native): ``%APPDATA%/tcl.ini``
    - MSYS2 / Cygwin: ``~/.config/tcl.ini``

    ``$XDG_CONFIG_HOME`` always takes precedence when set.
    """
    xdg = os.environ.get("XDG_CONFIG_HOME")
    if xdg:
        global_dir = Path(xdg)
    elif sys.platform == "win32" and not os.environ.get("MSYSTEM"):
        appdata = os.environ.get("APPDATA")
        global_dir = Path(appdata) if appdata else Path.home() / ".config"
    elif sys.platform == "darwin":
        global_dir = Path.home() / "Library" / "Application Support"
    else:
        global_dir = Path.home() / ".config"
    paths = [global_dir / "tcl.ini"]
    local = Path.cwd() / ".tcl.ini"
    if local.is_file():
        paths.append(local)
    return paths


def _load_config() -> _CliConfig:
    """Read ``~/.config/tcl.ini`` and optional ``.tcl.ini`` in CWD."""
    cp = configparser.ConfigParser()
    cp.read([str(p) for p in _config_file_paths()], encoding="utf-8")

    cfg = _CliConfig()

    # [output]
    if cp.has_section("output"):
        out = cp["output"]
        cfg.colour_mode = out.get("colour", cfg.colour_mode).strip().lower()
        try:
            cfg.tab_width = int(out.get("tabs", str(cfg.tab_width)))
        except ValueError:
            pass

    # [formatter]
    if cp.has_section("formatter"):
        fmt_dict: dict[str, Any] = {}
        for key, raw in cp.items("formatter"):
            if raw.lower() in ("true", "false"):
                fmt_dict[key] = raw.lower() == "true"
            else:
                try:
                    fmt_dict[key] = int(raw)
                except ValueError:
                    fmt_dict[key] = raw
        cfg.formatter = FormatterConfig.from_dict(fmt_dict)

    # [diagnostics]
    if cp.has_section("diagnostics"):
        sec = cp["diagnostics"]
        enabled = sec.get("enabled", "true").strip().lower()
        cfg.diagnostics_enabled = enabled != "false"
        cfg.disabled_diagnostics = set(_default_disabled_diagnostics())
        for code in _ALL_DIAGNOSTIC_CODES:
            default_val = "false" if code in cfg.disabled_diagnostics else "true"
            val = sec.get(code, default_val).strip().lower()
            if val == "false":
                cfg.disabled_diagnostics.add(code)
            elif val == "true":
                cfg.disabled_diagnostics.discard(code)

    # [optimiser]
    if cp.has_section("optimiser"):
        sec = cp["optimiser"]
        enabled = sec.get("enabled", "true").strip().lower()
        cfg.optimiser_enabled = enabled != "false"
        profile_raw = sec.get("profile", "").strip().lower()
        if profile_raw:
            cfg.optimiser_profile = profile_raw
        from shared.optimisation_profiles import (
            DEFAULT_ACTION_PROFILE,
            profile_from_name,
            profile_to_disabled,
        )

        try:
            profile = profile_from_name(cfg.optimiser_profile)
        except ValueError:
            profile = DEFAULT_ACTION_PROFILE
        cfg.disabled_optimisations = set(profile_to_disabled(profile))
        for code in _ALL_OPTIMISATION_CODES:
            raw_val = sec.get(code, "").strip().lower()
            if raw_val == "false":
                cfg.disabled_optimisations.add(code)
            elif raw_val == "true":
                cfg.disabled_optimisations.discard(code)

    return cfg


def _version_string() -> str:
    version = FULL_VERSION
    if BUILD_TIMESTAMP:
        version += f" ({BUILD_TIMESTAMP})"
    return version


def _infer_prog_name(argv0: str) -> str:
    raw_name = Path(argv0).name.strip()
    if not raw_name:
        return "tcl"

    stem = Path(raw_name).stem
    if not stem:
        return "tcl"

    lowered = stem.lower()
    if lowered.startswith("python"):
        return "tcl"
    if lowered.startswith("tcl-"):
        return "tcl"
    return stem


class _BriefHelpAction(argparse.Action):
    """Print a compact help overview and exit."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        prog = parser.prog
        lines = [
            f"{prog} <verb> [options] [inputs...]\n",
            "Verbs:",
        ]
        for name, aliases, desc in get_verb_catalogue():
            alias_str = f" ({aliases})" if aliases else ""
            label = f"{name}{alias_str}"
            lines.append(f"  {label:<26s} {desc}")
        lines.append("")
        lines.append("Common options:")
        lines.append("  --dialect DIALECT       Set dialect (default: tcl8.6)")
        lines.append("  --output, -o FILE       Output file (default: stdout)")
        lines.append("  --colour / --no-colour  Control syntax highlighting")
        lines.append("  --tabs N                Tab expansion width (default: 4)")
        lines.append("")
        config_paths = _config_file_paths()
        lines.append(f"Config: {config_paths[0]}")
        lines.append(f"Run '{prog} <verb> --help' for verb-specific help.")
        lines.append(f"Run '{prog} --help-all' for full option reference.")
        print("\n".join(lines))
        parser.exit()


class _FullHelpAction(argparse.Action):
    """Print the full help for every subcommand and exit."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        parser.print_help()
        print()

        for action in parser._subparsers._actions:  # noqa: SLF001
            if not isinstance(action, argparse._SubParsersAction):  # noqa: SLF001
                continue
            for name, subparser in action.choices.items():
                if name != subparser.prog.split()[-1]:
                    continue
                print(f"\n{'=' * 60}")
                print(f"  {name}")
                print(f"{'=' * 60}\n")
                subparser.print_help()
        parser.exit()


def build_parser(
    *,
    prog_name: str = "tcl",
    default_dialect: str = "tcl8.6",
) -> argparse.ArgumentParser:
    """Build the top-level argparse parser without consuming argv.

    Exposed so that :mod:`argcomplete` (and the ``completion`` verb that
    drives it) can introspect the verb tree without invoking
    ``parse_args``.
    """
    parser = argparse.ArgumentParser(
        prog=prog_name,
        description="Unified Tcl toolchain CLI.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
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

    # Register all @verb-decorated modules.
    from tooling.explorer.verbs import load_verbs

    load_verbs()
    apply_verb_registrations(sub, prog_name=prog_name, default_dialect=default_dialect)

    # Complex verb groups with sub-sub-commands (unchanged pattern).
    from tooling.explorer.verbs.docker import add_docker_subparser
    from tooling.explorer.verbs.pkg import add_pkg_subparser
    from tooling.explorer.verbs.venv import add_venv_subparser

    add_pkg_subparser(sub, prog_name=prog_name, default_dialect=default_dialect)
    add_venv_subparser(sub, prog_name=prog_name, default_dialect=default_dialect)
    add_docker_subparser(sub, prog_name=prog_name, default_dialect=default_dialect)

    return parser


def parse_args(
    argv: list[str],
    *,
    prog_name: str = "tcl",
    default_dialect: str = "tcl8.6",
) -> argparse.Namespace:
    parser = build_parser(prog_name=prog_name, default_dialect=default_dialect)
    # Hook argcomplete: when the shell invokes us with $_ARGCOMPLETE set,
    # this short-circuits, writes completions, and exits.  In normal use it
    # is a no-op.
    from tooling.explorer._argcomplete_support import autocomplete

    autocomplete(parser)
    return parser.parse_args(argv)


def main(
    argv: list[str] | None = None,
    *,
    prog_name: str | None = None,
) -> int:
    if argv is None:
        parsed_argv = sys.argv[1:]
        inferred_prog_name = _infer_prog_name(sys.argv[0])
    else:
        parsed_argv = argv
        inferred_prog_name = "tcl"

    selected_prog_name = prog_name or inferred_prog_name
    cli_config = _load_config()
    args = parse_args(
        parsed_argv,
        prog_name=selected_prog_name,
        default_dialect="tcl8.6",
    )
    args.cli_config = cli_config
    try:
        handler = args.handler
    except AttributeError as exc:  # pragma: no cover - argparse enforces this.
        raise RuntimeError("internal error: no command handler selected") from exc

    try:
        return handler(args)
    except TclCliError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
