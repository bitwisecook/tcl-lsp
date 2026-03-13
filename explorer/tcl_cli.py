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
- ``convert``: detect legacy patterns eligible for modernisation.
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
import difflib
import html
import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, cast

try:
    from lsp._build_info import BUILD_TIMESTAMP, FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"
    BUILD_TIMESTAMP = ""

from core.analysis.analyser import analyse
from core.analysis.semantic_graph import (
    build_call_graph,
    build_dataflow_graph,
    build_symbol_graph,
)
from core.analysis.semantic_model import Diagnostic, Severity
from core.commands.registry import REGISTRY
from core.commands.registry.info import lookup_command_info, lookup_event_info
from core.commands.registry.namespace_data import (
    event_multiplicity,
    order_events_for_file,
)
from core.commands.registry.runtime import configure_signatures
from core.compiler.cfg import build_cfg
from core.compiler.codegen import format_module_asm
from core.compiler.codegen.wasm import wasm_codegen_module
from core.compiler.lowering import lower_to_ir
from core.compiler.optimiser import apply_optimisations, find_optimisations
from core.diagram.extract import extract_diagram_data
from core.formatting import format_tcl
from core.formatting.config import FormatterConfig
from core.packages import PackageResolver
from core.parsing.command_segmenter import segment_commands
from core.parsing.lexer import TclLexer
from core.parsing.tokens import TokenType
from vm.compiler import compile_script

from .cli import main as explorer_main
from .formatters import range_dict
from .pipeline import AVAILABLE_DIALECTS, run_pipeline
from .serialise import serialise_result

_SOURCE_SUFFIXES = frozenset(
    {
        ".tcl",
        ".tk",
        ".itcl",
        ".tm",
        ".irul",
        ".irule",
        ".iapp",
        ".iappimpl",
        ".impl",
    }
)

_SKIP_DIRECTORY_NAMES = frozenset(
    {
        ".git",
        ".hg",
        ".svn",
        ".venv",
        "__pycache__",
        "node_modules",
        "build",
        "dist",
    }
)

_PROBLEM_SEVERITIES = frozenset({Severity.ERROR, Severity.WARNING})
_DIFF_LAYERS = ("ast", "ir", "cfg")
_CONVERTIBLE_CODES = frozenset(
    {
        "W100",
        "W104",
        "W110",
        "W304",
        "IRULE2001",
        "IRULE5001",
    }
)
_CONVERSION_MAP: dict[str, str] = {
    "W100": "Unbraced expr -> braced expr",
    "W104": "String concat for lists -> lappend",
    "W110": "== / != for strings -> eq / ne",
    "W304": "Missing -- option terminator -> add --",
    "IRULE2001": "Deprecated matchclass -> class match",
    "IRULE5001": "Ungated log in hot event -> add debug gating",
}
_WHEN_EVENT_PATTERN = re.compile(r"\bwhen\s+([A-Z_][A-Z0-9_]*)")
_HIGHLIGHT_TOKEN_KINDS = (
    "command",
    "subcommand",
    "comment",
    "variable",
    "command_subst",
    "braced",
    "expand",
)
_ANSI_HIGHLIGHT_CODES: dict[str, str] = {
    "command": "\033[1;34m",
    "subcommand": "\033[34m",
    "comment": "\033[90m",
    "variable": "\033[35m",
    "command_subst": "\033[36m",
    "braced": "\033[32m",
    "expand": "\033[33m",
}
_ANSI_RESET = "\033[0m"
_HTML_HIGHLIGHT_STYLES: dict[str, str] = {
    "command": "color:#2b6cb0;font-weight:600;",
    "subcommand": "color:#2c5282;",
    "comment": "color:#6b7280;",
    "variable": "color:#9f3ec7;",
    "command_subst": "color:#0f766e;",
    "braced": "color:#2f855a;",
    "expand": "color:#b7791f;",
}

# ---------------------------------------------------------------------------
# CLI configuration (INI file + flag cascade)
# ---------------------------------------------------------------------------

_ALL_DIAGNOSTIC_CODES = frozenset(
    {
        "E001",
        "E002",
        "E003",
        "E200",
        "W001",
        "W002",
        "W100",
        "W101",
        "W102",
        "W103",
        "W104",
        "W105",
        "W106",
        "W108",
        "W110",
        "W111",
        "W112",
        "W113",
        "W114",
        "W115",
        "W120",
        "W121",
        "W122",
        "W200",
        "W201",
        "H300",
        "W210",
        "W211",
        "W212",
        "W213",
        "W214",
        "W220",
        "W300",
        "W301",
        "W302",
        "W303",
        "W304",
        "W306",
        "W307",
        "W308",
        "W309",
    }
)

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
    disabled_optimisations: set[str] = field(default_factory=set)


def _config_file_paths() -> list[Path]:
    """Return INI config paths in read-order (global first, local last)."""
    xdg = os.environ.get("XDG_CONFIG_HOME")
    global_dir = Path(xdg) if xdg else Path.home() / ".config"
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
            # Booleans
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
        for code in _ALL_DIAGNOSTIC_CODES:
            val = sec.get(code, "true").strip().lower()
            if val == "false":
                cfg.disabled_diagnostics.add(code)

    # [optimiser]
    if cp.has_section("optimiser"):
        sec = cp["optimiser"]
        enabled = sec.get("enabled", "true").strip().lower()
        cfg.optimiser_enabled = enabled != "false"
        for code in _ALL_OPTIMISATION_CODES:
            val = sec.get(code, "true").strip().lower()
            if val == "false":
                cfg.disabled_optimisations.add(code)

    return cfg


_HELP_DIALECT_TERMS: dict[str, tuple[str, ...]] = {
    "eda-tools": (
        "eda",
        "asic",
        "fpga",
        "vivado",
        "quartus",
        "synopsys",
        "cadence",
    ),
    "f5-iapps": (
        "iapps",
        "iapp",
        "f5",
        "big-ip",
    ),
    "f5-irules": (
        "irules",
        "irule",
        "f5",
        "big-ip",
        "tmm",
        "event",
    ),
    "tcl8.4": (
        "tcl",
        "tk",
    ),
    "tcl8.5": (
        "tcl",
        "tk",
    ),
    "tcl8.6": (
        "tcl",
        "tk",
    ),
    "tcl9.0": (
        "tcl",
        "tk",
    ),
}


@dataclass(slots=True, frozen=True)
class InputDocument:
    """Resolved input document used by command verbs."""

    label: str
    source: str
    path: Path | None = None


class TclCliError(ValueError):
    """Raised when command-line input cannot be resolved."""


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
    if lowered.startswith("irule-"):
        return "irule"
    return stem


def _default_dialect_for_prog(prog_name: str) -> str:
    lowered = prog_name.lower()
    if lowered in {"irule", "irules"} or lowered.startswith("irule"):
        return "f5-irules"
    return "tcl8.6"


def _is_supported_source_file(path: Path) -> bool:
    if path.name == "pkgIndex.tcl":
        return True
    return path.suffix.lower() in _SOURCE_SUFFIXES


def _normalise_search_paths(paths: list[str]) -> list[str]:
    ordered: list[str] = []
    seen: set[str] = set()
    for raw in paths:
        expanded = os.path.abspath(os.path.expanduser(raw))
        if not os.path.isdir(expanded):
            continue
        if expanded in seen:
            continue
        seen.add(expanded)
        ordered.append(expanded)
    return ordered


def _extract_tcllib_paths() -> list[str]:
    raw = os.environ.get("TCLLIBPATH", "").strip()
    if not raw:
        return []
    # Tcl commonly stores this as a whitespace-separated list.
    # Accept ':' and ';' separators too for convenience.
    spaced = raw.replace(":", " ").replace(";", " ")
    return [token for token in spaced.split() if token]


def _iter_directory_sources(directory: Path, *, recursive: bool) -> list[Path]:
    files: list[Path] = []
    if recursive:
        for root, dir_names, file_names in os.walk(directory):
            dir_names[:] = sorted(
                name
                for name in dir_names
                if name not in _SKIP_DIRECTORY_NAMES and not name.startswith(".")
            )
            for file_name in sorted(file_names):
                path = Path(root) / file_name
                if _is_supported_source_file(path):
                    files.append(path.resolve())
    else:
        for path in sorted(directory.iterdir()):
            if path.is_file() and _is_supported_source_file(path):
                files.append(path.resolve())
    return files


def _resolve_package_sources(
    package_names: list[str],
    *,
    package_paths: list[str],
) -> dict[str, list[Path]]:
    if not package_names:
        return {}

    resolver = PackageResolver()
    resolver.configure(package_paths)
    resolver.scan_packages()

    resolved: dict[str, list[Path]] = {}
    missing: list[str] = []
    for package_name in package_names:
        source_files = resolver.resolve(package_name)
        if not source_files:
            missing.append(package_name)
            continue
        resolved[package_name] = [Path(path).resolve() for path in source_files]

    if missing:
        scanned = ", ".join(package_paths) if package_paths else "(none)"
        names = ", ".join(missing)
        raise TclCliError(f"package(s) not found: {names}. Scanned package paths: {scanned}")

    return resolved


def _read_input_documents(
    inputs: list[str],
    *,
    inline_sources: list[str],
    package_paths: list[str],
    recursive: bool,
) -> list[InputDocument]:
    file_paths: list[Path] = []
    ordered_inputs: list[Path | str] = []
    package_names: list[str] = []
    search_paths: list[str] = [os.getcwd(), *package_paths, *_extract_tcllib_paths()]

    for raw_input in inputs:
        path = Path(raw_input).expanduser()
        if not path.exists():
            package_names.append(raw_input)
            ordered_inputs.append(raw_input)
            continue

        if path.is_file():
            if not _is_supported_source_file(path):
                raise TclCliError(
                    f"unsupported source file: {path} (expected Tcl/iRules file extensions)"
                )
            resolved_path = path.resolve()
            ordered_inputs.append(resolved_path)
            search_paths.append(str(resolved_path.parent))
            continue

        if path.is_dir():
            search_paths.append(str(path.resolve()))
            discovered = _iter_directory_sources(path.resolve(), recursive=recursive)
            if not discovered:
                raise TclCliError(f"directory has no supported Tcl source files: {path}")
            ordered_inputs.extend(discovered)
            continue

        raise TclCliError(f"unsupported input path type: {path}")

    resolved_package_paths = _normalise_search_paths(search_paths)
    resolved_packages = _resolve_package_sources(
        package_names,
        package_paths=resolved_package_paths,
    )
    for entry in ordered_inputs:
        if isinstance(entry, Path):
            file_paths.append(entry)
        else:
            file_paths.extend(resolved_packages[entry])

    documents: list[InputDocument] = []
    for index, source_text in enumerate(inline_sources, start=1):
        label = f"<inline:{index}>"
        documents.append(InputDocument(label=label, source=source_text, path=None))

    seen_paths: set[str] = set()
    for file_path in file_paths:
        key = str(file_path)
        if key in seen_paths:
            continue
        seen_paths.add(key)
        try:
            source = file_path.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            raise TclCliError(f"failed to read {file_path}: {exc}") from exc
        documents.append(InputDocument(label=str(file_path), source=source, path=file_path))

    if not documents and not sys.stdin.isatty():
        documents.append(InputDocument(label="<stdin>", source=sys.stdin.read(), path=None))

    if not documents:
        raise TclCliError(
            "no input provided; pass files/directories/packages, --source, or pipe stdin"
        )

    return documents


def _combine_sources(documents: list[InputDocument]) -> str:
    chunks = [document.source.rstrip("\n") for document in documents]
    return "\n\n".join(chunks)


def _write_text_output(output_path: str, text: str) -> None:
    if output_path == "-":
        sys.stdout.write(text)
        if text and not text.endswith("\n"):
            sys.stdout.write("\n")
        return
    Path(output_path).write_text(text, encoding="utf-8")


def _add_colour_arguments(parser: argparse.ArgumentParser) -> None:
    """Add --colour/--no-colour and --tabs flags to a subcommand parser."""
    parser.add_argument(
        "--no-colour",
        "--no-color",
        dest="no_colour",
        action="store_true",
        help="Disable syntax highlighting on stdout.",
    )
    parser.add_argument(
        "--colour",
        "--color",
        dest="force_colour",
        action="store_true",
        help="Force syntax highlighting even when stdout is not a TTY.",
    )
    parser.add_argument(
        "--tabs",
        type=int,
        default=None,
        metavar="N",
        help="Expand tabs to N spaces on stdout (default: 4, 0 to keep tabs).",
    )


def _add_toggle_arguments(parser: argparse.ArgumentParser, *, kind: str) -> None:
    """Add --disable/--enable CODE flags for diagnostics or optimisations."""
    parser.add_argument(
        "--disable",
        default=None,
        metavar="CODE[,CODE,...]",
        help=f"Suppress specific {kind} codes (comma-separated).",
    )
    parser.add_argument(
        "--enable",
        default=None,
        metavar="CODE[,CODE,...]",
        help=f"Re-enable {kind} codes disabled in config (comma-separated).",
    )


def _parse_code_set(raw: str | None) -> set[str]:
    """Parse a comma-separated list of codes into a set."""
    if not raw:
        return set()
    return {c.strip().upper() for c in raw.split(",") if c.strip()}


def _resolve_disabled_diagnostics(args: argparse.Namespace) -> set[str]:
    """Build the set of diagnostic codes to suppress."""
    cfg: _CliConfig | None = getattr(args, "cli_config", None)
    disabled = set(cfg.disabled_diagnostics) if cfg else set()
    disabled |= _parse_code_set(getattr(args, "disable", None))
    disabled -= _parse_code_set(getattr(args, "enable", None))
    return disabled


def _resolve_disabled_optimisations(args: argparse.Namespace) -> set[str]:
    """Build the set of optimisation codes to suppress."""
    cfg: _CliConfig | None = getattr(args, "cli_config", None)
    disabled = set(cfg.disabled_optimisations) if cfg else set()
    disabled |= _parse_code_set(getattr(args, "disable", None))
    disabled -= _parse_code_set(getattr(args, "enable", None))
    return disabled


def _add_formatter_arguments(parser: argparse.ArgumentParser) -> None:
    """Add commonly-used formatter knobs to a subcommand parser."""
    parser.add_argument(
        "--indent-size",
        type=int,
        default=None,
        metavar="N",
        help="Spaces per indent level (default: 4).",
    )
    parser.add_argument(
        "--indent-style",
        choices=("spaces", "tabs"),
        default=None,
        help="Indent using spaces or tabs (default: spaces).",
    )
    parser.add_argument(
        "--max-line-length",
        type=int,
        default=None,
        metavar="N",
        help="Hard line-length limit (default: 120).",
    )
    parser.add_argument(
        "--goal-line-length",
        type=int,
        default=None,
        metavar="N",
        help="Soft target line length (default: 100).",
    )
    parser.add_argument(
        "--expand-bodies",
        action="store_true",
        default=None,
        help="Expand compact single-line bodies.",
    )
    parser.add_argument(
        "--no-semicolons",
        action="store_true",
        default=None,
        help="Replace semicolons with newlines (enabled by default).",
    )
    parser.add_argument(
        "--keep-semicolons",
        action="store_true",
        default=None,
        help="Keep semicolons as-is (do not replace with newlines).",
    )


def _resolve_formatter_config(args: argparse.Namespace) -> FormatterConfig:
    """Build a FormatterConfig from config file + CLI flag overrides."""
    cfg: _CliConfig | None = getattr(args, "cli_config", None)
    base = cfg.formatter if cfg else FormatterConfig()

    from core.formatting.config import IndentStyle

    overrides: dict[str, Any] = {}
    if getattr(args, "indent_size", None) is not None:
        overrides["indent_size"] = args.indent_size
    if getattr(args, "indent_style", None) is not None:
        overrides["indent_style"] = (
            IndentStyle.TABS if args.indent_style == "tabs" else IndentStyle.SPACES
        )
    if getattr(args, "max_line_length", None) is not None:
        overrides["max_line_length"] = args.max_line_length
    if getattr(args, "goal_line_length", None) is not None:
        overrides["goal_line_length"] = args.goal_line_length
    if getattr(args, "expand_bodies", None):
        overrides["expand_single_line_bodies"] = True
    if getattr(args, "no_semicolons", None):
        overrides["replace_semicolons_with_newlines"] = True
    if getattr(args, "keep_semicolons", None):
        overrides["replace_semicolons_with_newlines"] = False

    return base.replace(**overrides) if overrides else base


def _resolve_use_colour(args: argparse.Namespace) -> bool:
    """Decide whether ANSI syntax highlighting should be applied.

    Priority: CLI flag > config file > auto-detect.
    """
    # Explicit CLI flags always win.
    if getattr(args, "force_colour", False):
        return True
    if getattr(args, "no_colour", False):
        return False
    # Config file setting.
    cfg: _CliConfig | None = getattr(args, "cli_config", None)
    if cfg is not None:
        if cfg.colour_mode == "always":
            return True
        if cfg.colour_mode == "never":
            return False
    # Auto-detect: colour only when writing to a TTY.
    return getattr(args, "output", "-") == "-" and sys.stdout.isatty()


def _resolve_tab_width(args: argparse.Namespace) -> int:
    """Return the effective tab expansion width."""
    # CLI --tabs flag (argparse default is 4).
    cli_val = getattr(args, "tabs", None)
    if cli_val is not None:
        return cli_val
    cfg: _CliConfig | None = getattr(args, "cli_config", None)
    if cfg is not None:
        return cfg.tab_width
    return 4


def _write_highlighted_output(
    output_path: str, text: str, *, use_colour: bool, tab_width: int = 4
) -> None:
    """Write Tcl source to *output_path*, optionally syntax-highlighted."""
    if output_path == "-" and tab_width > 0:
        text = text.expandtabs(tab_width)
    if use_colour:
        text = _highlight_source_ansi(text, use_colour=True)
    _write_text_output(output_path, text)


def _write_binary_output(output_path: str, payload: bytes) -> None:
    if output_path == "-":
        sys.stdout.buffer.write(payload)
        sys.stdout.buffer.flush()
        return
    Path(output_path).write_bytes(payload)


def _format_diagnostic_line(document: InputDocument, diagnostic: Diagnostic) -> str:
    line = diagnostic.range.start.line + 1
    column = diagnostic.range.start.character + 1
    severity = diagnostic.severity.name.lower()
    code = diagnostic.code or "-"
    return f"{document.label}:{line}:{column}: {severity:<7} {code:<8} {diagnostic.message}"


def _add_input_arguments(
    parser: argparse.ArgumentParser,
    *,
    include_output: bool = False,
    default_dialect: str,
) -> None:
    parser.add_argument(
        "inputs",
        nargs="*",
        help="Input files, directories, or package names.",
    )
    parser.add_argument(
        "--source",
        action="append",
        default=[],
        help="Inline Tcl source text (can be repeated).",
    )
    parser.add_argument(
        "--package-path",
        action="append",
        default=[],
        help="Additional directory to scan for pkgIndex.tcl package metadata.",
    )
    parser.add_argument(
        "--no-recursive",
        action="store_true",
        help="Do not recurse when an input is a directory.",
    )
    parser.add_argument(
        "--dialect",
        choices=AVAILABLE_DIALECTS,
        default=default_dialect,
        help=(f"Dialect profile for analysis/compile steps (default: {default_dialect})."),
    )
    if include_output:
        parser.add_argument(
            "--output",
            "-o",
            default="-",
            help="Output path ('-' for stdout).",
        )


def _run_opt(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    disabled = _resolve_disabled_optimisations(args)
    all_optimisations = find_optimisations(source)
    optimisations = [o for o in all_optimisations if o.code not in disabled]
    optimised_source = apply_optimisations(source, optimisations)

    # Summary: append a styled comment block when writing to stdout,
    # otherwise emit a one-liner on stderr.
    if args.output == "-" and optimisations:
        lines = [
            "\n\n# -------------",
            f"# optimised: {len(optimisations)} rewrite(s)",
        ]
        for opt in optimisations:
            lines.append(f"# {opt.code}  {opt.message}")
        optimised_source = optimised_source.rstrip("\n") + "\n" + "\n".join(lines) + "\n"

    _write_highlighted_output(
        args.output,
        optimised_source,
        use_colour=_resolve_use_colour(args),
        tab_width=_resolve_tab_width(args),
    )

    if args.output != "-":
        print(
            f"optimised {len(documents)} input(s); rewrites={len(optimisations)}",
            file=sys.stderr,
        )
    return 0


def _run_diag(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    report: list[dict[str, Any]] = []
    problem_count = 0
    diagnostic_count = 0

    disabled = _resolve_disabled_diagnostics(args)

    for document in documents:
        diagnostics = [d for d in analyse(document.source).diagnostics if d.code not in disabled]
        if diagnostics:
            diagnostic_count += len(diagnostics)
            problem_count += sum(1 for diag in diagnostics if diag.severity in _PROBLEM_SEVERITIES)
        report.append(
            {
                "file": document.label,
                "diagnostics": [
                    {
                        "line": diag.range.start.line + 1,
                        "column": diag.range.start.character + 1,
                        "severity": diag.severity.name.lower(),
                        "code": diag.code or "",
                        "message": diag.message,
                    }
                    for diag in diagnostics
                ],
            }
        )

    if args.json:
        print(json.dumps(report, indent=2))
    else:
        for item in report:
            diagnostics = cast(list[dict[str, Any]], item["diagnostics"])
            for diag in diagnostics:
                print(
                    f"{item['file']}:{diag['line']}:{diag['column']}: "
                    f"{diag['severity']:<7} {diag['code'] or '-':<8} {diag['message']}"
                )
        if diagnostic_count == 0:
            print("no diagnostics")

    print(
        f"diagnostics={diagnostic_count} across {len(documents)} input(s)",
        file=sys.stderr,
    )
    return 1 if problem_count else 0


def _run_validate(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    disabled = _resolve_disabled_diagnostics(args)
    errors: list[tuple[InputDocument, Diagnostic]] = []
    for document in documents:
        diagnostics = analyse(document.source).diagnostics
        errors.extend(
            (document, diag)
            for diag in diagnostics
            if diag.severity is Severity.ERROR and diag.code not in disabled
        )

    if args.json:
        payload = {
            "ok": not errors,
            "inputs": len(documents),
            "error_count": len(errors),
            "errors": [
                {
                    "file": document.label,
                    "line": diagnostic.range.start.line + 1,
                    "column": diagnostic.range.start.character + 1,
                    "severity": diagnostic.severity.name.lower(),
                    "code": diagnostic.code or "",
                    "message": diagnostic.message,
                }
                for document, diagnostic in errors
            ],
        }
        print(json.dumps(payload, indent=2))
        return 1 if errors else 0

    if errors:
        for document, diagnostic in errors:
            print(_format_diagnostic_line(document, diagnostic))
        print(f"validation failed: {len(errors)} error(s)", file=sys.stderr)
        return 1

    print("validation ok", file=sys.stderr)
    return 0


def _run_format(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    fmt_cfg = _resolve_formatter_config(args)
    formatted_source = format_tcl(source, config=fmt_cfg)
    _write_highlighted_output(
        args.output,
        formatted_source,
        use_colour=_resolve_use_colour(args),
        tab_width=_resolve_tab_width(args),
    )
    return 0


def _run_minify(args: argparse.Namespace) -> int:
    from core.minifier import minify_tcl

    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    use_colour = _resolve_use_colour(args)

    if args.aggressive:
        result = minify_tcl(source, aggressive=True)
        _write_highlighted_output(
            args.output, result.source, use_colour=use_colour, tab_width=_resolve_tab_width(args)
        )
        if args.symbol_map:
            _write_text_output(args.symbol_map, result.symbol_map.format())
        if sys.stderr.isatty():
            pct = f"{result.savings_pct:.1f}"
            print(
                f"{result.original_length} → {result.minified_length} chars "
                f"({pct}% reduction, {result.optimisations_applied} optimisations)",
                file=sys.stderr,
            )
    elif args.compact:
        minified, symbol_map = minify_tcl(source, compact_names=True)
        _write_highlighted_output(
            args.output, minified, use_colour=use_colour, tab_width=_resolve_tab_width(args)
        )
        if args.symbol_map:
            _write_text_output(args.symbol_map, symbol_map.format())
        elif sys.stderr.isatty():
            map_text = symbol_map.format()
            if map_text:
                print(map_text, file=sys.stderr)
    else:
        minified = minify_tcl(source)
        _write_highlighted_output(
            args.output, minified, use_colour=use_colour, tab_width=_resolve_tab_width(args)
        )
    return 0


def _run_unminify_error(args: argparse.Namespace) -> int:
    from core.minifier import unminify_error

    # Read symbol map
    symbol_map_text = Path(args.symbol_map).read_text(encoding="utf-8")

    # Read error message
    if args.error:
        error_text = args.error
    elif args.error_file:
        if args.error_file == "-":
            error_text = sys.stdin.read()
        else:
            error_text = Path(args.error_file).read_text(encoding="utf-8")
    else:
        print("error: provide --error TEXT or --error-file FILE", file=sys.stderr)
        return 1

    # Optional source files for line-number remapping
    minified_source = None
    original_source = None
    if args.minified:
        minified_source = Path(args.minified).read_text(encoding="utf-8")
    if args.original:
        original_source = Path(args.original).read_text(encoding="utf-8")

    translated = unminify_error(
        error_text,
        symbol_map=symbol_map_text,
        minified_source=minified_source,
        original_source=original_source,
    )
    _write_text_output(args.output, translated)
    return 0


def _detect_event_entries(source: str) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    for match in _WHEN_EVENT_PATTERN.finditer(source):
        name = match.group(1)
        if name in seen:
            continue
        seen.add(name)
        line = source[: match.start()].count("\n") + 1
        entries.append(
            {
                "kind": "event",
                "name": name,
                "line": line,
                "depth": 0,
            }
        )
    return entries


def _collect_scope_symbol_entries(scope, *, depth: int = 0) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []

    for proc in scope.procs.values():
        params = [param.name for param in proc.params]
        line = proc.name_range.start.line + 1 if proc.name_range else None
        entries.append(
            {
                "kind": "function",
                "name": proc.name,
                "line": line,
                "depth": depth,
                "params": params,
            }
        )

    if scope.kind in ("global", "namespace"):
        for variable in scope.variables.values():
            if variable.definition_range is None:
                continue
            entries.append(
                {
                    "kind": "variable",
                    "name": variable.name,
                    "line": variable.definition_range.start.line + 1,
                    "depth": depth,
                }
            )

    for child in scope.children:
        if child.kind == "namespace" and child.body_range is not None:
            entries.append(
                {
                    "kind": "namespace",
                    "name": child.name,
                    "line": child.body_range.start.line + 1,
                    "depth": depth,
                }
            )
            entries.extend(_collect_scope_symbol_entries(child, depth=depth + 1))
        elif child.kind == "proc":
            entries.extend(_collect_scope_symbol_entries(child, depth=depth + 1))

    return entries


def _run_symbols(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    analysis = analyse(source)
    event_entries = _detect_event_entries(source)
    scope_entries = _collect_scope_symbol_entries(analysis.global_scope)
    entries = [*event_entries, *scope_entries]

    payload = {
        "count": len(entries),
        "dialect": args.dialect,
        "inputs": [document.label for document in documents],
        "symbols": entries,
    }
    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0

    if not entries:
        _write_text_output(args.output, "no symbols")
        return 0

    lines: list[str] = [f"symbols: {len(entries)}"]
    for entry in entries:
        depth_raw = entry.get("depth", 0)
        depth = depth_raw if isinstance(depth_raw, int) else 0
        indent = "  " * depth
        kind = str(entry.get("kind", "?"))
        name = str(entry.get("name", "?"))
        line = entry.get("line")
        line_suffix = f" (line {line})" if isinstance(line, int) else ""
        if kind == "function":
            params_raw = entry.get("params", [])
            params = (
                ", ".join(str(item) for item in params_raw) if isinstance(params_raw, list) else ""
            )
            lines.append(f"{indent}function {name}({params}){line_suffix}")
        else:
            lines.append(f"{indent}{kind} {name}{line_suffix}")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_diagram(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    data = extract_diagram_data(source)

    if args.json:
        _write_text_output(args.output, json.dumps(data, indent=2))
        return 1 if "error" in data else 0

    if "error" in data:
        _write_text_output(args.output, f"diagram error: {data['error']}")
        return 1

    events = data.get("events", [])
    procedures = data.get("procedures", [])
    lines = [
        f"diagram: events={len(events)} procedures={len(procedures)}",
    ]
    if events:
        lines.append("events:")
        for event in events:
            flow_count = len(event.get("flow", []))
            multiplicity = str(event.get("multiplicity", "unknown"))
            lines.append(f"  {event.get('name', '?')} ({multiplicity}) nodes={flow_count}")
    if procedures:
        lines.append("procedures:")
        for proc in procedures:
            params = ", ".join(str(item) for item in proc.get("params", []))
            flow_count = len(proc.get("flow", []))
            lines.append(f"  {proc.get('name', '?')}({params}) nodes={flow_count}")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_callgraph(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)
    data = build_call_graph(source)

    if args.json:
        _write_text_output(args.output, json.dumps(data, indent=2))
        return 0

    nodes = data.get("nodes", [])
    edges = data.get("edges", [])
    roots = data.get("roots", [])
    leaves = data.get("leaf_procs", [])

    lines = [f"call graph: procs={len(nodes)} edges={len(edges)}"]
    if nodes:
        lines.append("procs:")
        for node in nodes:
            params = ", ".join(str(item) for item in node.get("params", []))
            line = node.get("line")
            line_suffix = f" (line {line + 1})" if isinstance(line, int) else ""
            pure_suffix = " [pure]" if node.get("pure") else ""
            lines.append(f"  {node['name']}({params}){line_suffix}{pure_suffix}")
    if edges:
        lines.append("edges:")
        for edge in edges:
            lines.append(f"  {edge['caller']} -> {edge['callee']}")
    if roots:
        lines.append(f"roots: {', '.join(str(item) for item in roots)}")
    if leaves:
        lines.append(f"leaves: {', '.join(str(item) for item in leaves)}")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _append_symbolgraph_scope(lines: list[str], scope: dict[str, Any], *, depth: int = 0) -> None:
    indent = "  " * depth
    kind = str(scope.get("kind", "?"))
    name = str(scope.get("name", "?"))
    lines.append(f"{indent}{kind} {name}")

    procs = scope.get("procs")
    if isinstance(procs, list):
        for proc in procs:
            if not isinstance(proc, dict):
                continue
            proc_dict = cast(dict[str, Any], proc)
            params_raw = proc_dict.get("params")
            params = (
                ", ".join(str(item) for item in params_raw) if isinstance(params_raw, list) else ""
            )
            line = proc_dict.get("line")
            line_suffix = f" (line {line + 1})" if isinstance(line, int) else ""
            refs = proc_dict.get("ref_count", 0)
            lines.append(
                f"{indent}  proc {proc_dict.get('name', '?')}({params}){line_suffix} [{refs} refs]"
            )

    variables = scope.get("variables")
    if isinstance(variables, list):
        for variable in variables:
            if not isinstance(variable, dict):
                continue
            variable_dict = cast(dict[str, Any], variable)
            line = variable_dict.get("line")
            line_suffix = f" (line {line + 1})" if isinstance(line, int) else ""
            refs_raw = variable_dict.get("references")
            refs = len(refs_raw) if isinstance(refs_raw, list) else 0
            lines.append(
                f"{indent}  var {variable_dict.get('name', '?')}{line_suffix} [{refs} refs]"
            )

    children = scope.get("children")
    if isinstance(children, list):
        for child in children:
            if isinstance(child, dict):
                _append_symbolgraph_scope(lines, cast(dict[str, Any], child), depth=depth + 1)


def _run_symbolgraph(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)
    data = build_symbol_graph(source)

    if args.json:
        _write_text_output(args.output, json.dumps(data, indent=2))
        return 0

    summary = data.get("summary", {})
    total_procs = summary.get("total_procs", 0)
    total_variables = summary.get("total_variables", 0)
    total_namespaces = summary.get("total_namespaces", 0)

    lines = [
        f"symbol graph: procs={total_procs} variables={total_variables} namespaces={total_namespaces}"
    ]
    scopes = data.get("scopes", [])
    if scopes:
        lines.append("scopes:")
        for scope in scopes:
            if isinstance(scope, dict):
                _append_symbolgraph_scope(lines, scope, depth=1)
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_dataflow(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)
    data = build_dataflow_graph(source)

    if args.json:
        _write_text_output(args.output, json.dumps(data, indent=2))
        return 0

    summary = data.get("summary", {})
    lines = [
        "dataflow:"
        f" taintWarnings={summary.get('total_taint_warnings', 0)}"
        f" taintedVars={summary.get('tainted_variable_count', 0)}"
        f" pure={summary.get('pure_proc_count', 0)}"
        f" impure={summary.get('impure_proc_count', 0)}"
    ]

    warnings = data.get("taint_warnings", [])
    if warnings:
        lines.append("taint warnings:")
        for warning in warnings:
            if not isinstance(warning, dict):
                continue
            line = warning.get("line")
            line_no = line + 1 if isinstance(line, int) else "?"
            code = warning.get("code", "")
            message = warning.get("message", "")
            lines.append(f"  {code} line {line_no}: {message}")

    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_event_order(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)

    ordered = order_events_for_file(source)
    events = [
        {
            "index": index,
            "name": event_name,
            "multiplicity": event_multiplicity(event_name),
        }
        for index, event_name in enumerate(ordered, start=1)
    ]
    payload = {
        "count": len(events),
        "dialect": args.dialect,
        "events": events,
    }
    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0

    lines = [f"event order: {len(events)} event(s)"]
    for item in events:
        lines.append(f"  {item['index']}. {item['name']} ({item['multiplicity']})")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_event_info(args: argparse.Namespace) -> int:
    info = lookup_event_info(args.event, dialect="f5-irules")
    payload = {
        "event": info.event,
        "known": info.known,
        "deprecated": info.deprecated,
        "multiplicity": info.multiplicity,
        "description": info.description,
        "side": info.side,
        "transport": info.transport,
        "impliedProfiles": list(info.implied_profiles),
        "validCommandCount": info.valid_command_count,
        "validCommands": list(info.valid_commands),
    }

    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0 if info.known else 1

    lines = [
        f"event: {info.event}",
        f"known: {'yes' if info.known else 'no'}",
        f"deprecated: {'yes' if payload['deprecated'] else 'no'}",
        f"multiplicity: {payload['multiplicity']}",
    ]
    if info.description:
        lines.append(f"description: {info.description}")
    lines.append(f"side: {payload['side']}")
    if payload["transport"]:
        lines.append(f"transport: {payload['transport']}")
    if payload["impliedProfiles"]:
        lines.append(f"profiles: {', '.join(str(item) for item in payload['impliedProfiles'])}")
    lines.append(f"valid commands: {info.valid_command_count}")
    _write_text_output(args.output, "\n".join(lines))
    return 0 if info.known else 1


def _run_command_info(args: argparse.Namespace) -> int:
    try:
        info = lookup_command_info(args.command, dialect=args.dialect)
    except ValueError as exc:
        raise TclCliError(str(exc)) from exc

    if not info.found:
        payload = {
            "found": False,
            "command": args.command.strip(),
            "dialect": args.dialect,
        }
        if args.json:
            _write_text_output(args.output, json.dumps(payload, indent=2))
        else:
            _write_text_output(
                args.output,
                f"command not found: {args.command.strip()} (dialect={args.dialect})",
            )
        return 1

    payload = {
        "found": True,
        "command": info.command,
        "dialect": info.dialect,
        "summary": info.summary,
        "synopsis": list(info.synopsis),
        "switches": list(info.switches),
        "validEvents": list(info.valid_events),
    }
    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0

    lines = [f"command: {info.command}", f"dialect: {args.dialect}"]
    if payload["summary"]:
        lines.append(f"summary: {payload['summary']}")
    if payload["synopsis"]:
        lines.extend(f"synopsis: {item}" for item in payload["synopsis"])
    if info.switches:
        lines.append(f"switches: {', '.join(info.switches)}")
    if info.valid_events:
        lines.append(
            f"valid events ({len(info.valid_events)}): {', '.join(info.valid_events[:20])}"
        )
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_convert(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)

    diagnostics = analyse(source).diagnostics
    convertible = [diag for diag in diagnostics if (diag.code or "") in _CONVERTIBLE_CODES]
    issues = [
        {
            "code": diag.code or "",
            "line": diag.range.start.line + 1,
            "column": diag.range.start.character + 1,
            "message": diag.message,
            "conversion": _CONVERSION_MAP.get(diag.code or "", "modernise"),
        }
        for diag in convertible
    ]
    payload = {
        "count": len(issues),
        "dialect": args.dialect,
        "issues": issues,
    }
    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0

    if not issues:
        _write_text_output(args.output, "no legacy patterns detected")
        return 0

    lines = [f"legacy patterns: {len(issues)}"]
    for issue in issues:
        lines.append(f"  {issue['code']} line {issue['line']}:{issue['column']} {issue['message']}")
        lines.append(f"    conversion: {issue['conversion']}")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_dis(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    module_asm, _ = compile_script(source, optimise=args.optimise)
    disassembly = format_module_asm(module_asm)
    _write_text_output(args.output, disassembly)
    return 0


def _run_compwasm(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    ir_module = lower_to_ir(source)
    cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=args.optimise)
    wasm_bytes = wasm_module.to_bytes()

    _write_binary_output(args.output, wasm_bytes)
    if args.wat_output:
        _write_text_output(args.wat_output, wasm_module.to_wat())

    output_target = "stdout" if args.output == "-" else args.output
    print(
        f"wrote wasm binary ({len(wasm_bytes)} bytes) to {output_target}",
        file=sys.stderr,
    )
    return 0


def _highlight_token_kind(
    token_type: TokenType,
    span: tuple[int, int],
    *,
    command_spans: set[tuple[int, int]],
    subcommand_spans: set[tuple[int, int]],
) -> str | None:
    if token_type is TokenType.COMMENT:
        return "comment"
    if token_type is TokenType.VAR:
        return "variable"
    if token_type is TokenType.CMD:
        return "command_subst"
    if token_type is TokenType.STR:
        return "braced"
    if token_type is TokenType.EXPAND:
        return "expand"
    if token_type is TokenType.ESC:
        if span in command_spans:
            return "command"
        if span in subcommand_spans:
            return "subcommand"
    return None


def _collect_command_spans(source: str) -> tuple[set[tuple[int, int]], set[tuple[int, int]]]:
    command_spans: set[tuple[int, int]] = set()
    subcommand_spans: set[tuple[int, int]] = set()
    for command in segment_commands(source, registry_snapshot=REGISTRY, recovery=False):
        if command.argv:
            first = command.argv[0]
            command_spans.add((first.start.offset, first.end.offset))
        if command.subcommand and len(command.argv) > 1:
            second = command.argv[1]
            subcommand_spans.add((second.start.offset, second.end.offset))
    return command_spans, subcommand_spans


def _is_body_token(token_text: str) -> bool:
    """Return True if a braced string looks like a command body.

    The Tcl lexer emits STR tokens that start with ``{`` and include
    the content up to (but not always including) the closing ``}``.
    We detect bodies by checking for newlines or semicolons in the
    inner content.
    """
    if not token_text.startswith("{"):
        return False
    # Strip the opening brace; closing brace may or may not be present.
    inner = token_text[1:].rstrip("}")
    return "\n" in inner or ";" in inner


def _highlight_source_ansi(source: str, *, use_colour: bool, _depth: int = 0) -> str:
    if not source:
        return source
    if not use_colour:
        return source
    if _depth > 8:
        return source

    lexer = TclLexer(source)
    tokens = lexer.tokenise_all()
    command_spans, subcommand_spans = _collect_command_spans(source)

    chunks: list[str] = []
    cursor = 0
    for token in tokens:
        start = token.start.offset
        end = token.end.offset + 1
        if end <= start:
            continue

        if start > cursor:
            chunks.append(source[cursor:start])

        span = (token.start.offset, token.end.offset)
        token_text = source[start:end]

        # Recursively highlight bodies (braced strings containing commands).
        if token.type is TokenType.STR and _is_body_token(token_text):
            # Strip the opening brace and optional closing brace.
            if token_text.endswith("}"):
                inner = token_text[1:-1]
                suffix = "}"
            else:
                inner = token_text[1:]
                suffix = ""
            highlighted_inner = _highlight_source_ansi(inner, use_colour=True, _depth=_depth + 1)
            chunks.append("{" + highlighted_inner + suffix)
            cursor = end
            continue

        kind = _highlight_token_kind(
            token.type,
            span,
            command_spans=command_spans,
            subcommand_spans=subcommand_spans,
        )
        if kind is None:
            chunks.append(token_text)
        else:
            code = _ANSI_HIGHLIGHT_CODES.get(kind, "")
            chunks.append(f"{code}{token_text}{_ANSI_RESET}")
        cursor = end

    if cursor < len(source):
        chunks.append(source[cursor:])
    return "".join(chunks)


def _highlight_source_html(source: str) -> str:
    if not source:
        return "<pre></pre>\n"

    lexer = TclLexer(source)
    tokens = lexer.tokenise_all()
    command_spans, subcommand_spans = _collect_command_spans(source)

    chunks: list[str] = []
    cursor = 0
    for token in tokens:
        start = token.start.offset
        end = token.end.offset + 1
        if end <= start:
            continue

        if start > cursor:
            chunks.append(html.escape(source[cursor:start]))

        span = (token.start.offset, token.end.offset)
        kind = _highlight_token_kind(
            token.type,
            span,
            command_spans=command_spans,
            subcommand_spans=subcommand_spans,
        )
        token_text = html.escape(source[start:end])
        if kind is None:
            chunks.append(token_text)
        else:
            style = _HTML_HIGHLIGHT_STYLES.get(kind, "")
            chunks.append(f'<span style="{style}">{token_text}</span>')
        cursor = end

    if cursor < len(source):
        chunks.append(html.escape(source[cursor:]))

    return "<pre>\n" + "".join(chunks) + "\n</pre>\n"


def _run_highlight(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)

    if args.format == "html":
        output = _highlight_source_html(source)
    else:
        output = _highlight_source_ansi(source, use_colour=_resolve_use_colour(args))

    tw = _resolve_tab_width(args)
    if args.output == "-" and tw > 0:
        output = output.expandtabs(tw)
    _write_text_output(args.output, output)
    return 0


def _parse_diff_layers(raw_value: str) -> tuple[str, ...]:
    selected: list[str] = []
    seen: set[str] = set()
    for token in raw_value.split(","):
        name = token.strip().lower()
        if not name:
            continue
        if name == "all":
            for layer in _DIFF_LAYERS:
                if layer not in seen:
                    seen.add(layer)
                    selected.append(layer)
            continue
        if name not in _DIFF_LAYERS:
            valid = ", ".join((*_DIFF_LAYERS, "all"))
            raise TclCliError(f"unknown diff layer {name!r}; choose from: {valid}")
        if name not in seen:
            seen.add(name)
            selected.append(name)

    if not selected:
        raise TclCliError("no diff layers selected; pass --show ast,ir,cfg or --show all")
    return tuple(selected)


def _read_diff_side(
    input_value: str,
    *,
    inline_sources: list[str],
    package_paths: list[str],
    recursive: bool,
) -> tuple[str, list[str]]:
    documents = _read_input_documents(
        [input_value],
        inline_sources=inline_sources,
        package_paths=package_paths,
        recursive=recursive,
    )
    return _combine_sources(documents), [document.label for document in documents]


def _serialise_command_ast(source: str) -> dict[str, object]:
    commands = segment_commands(source, registry_snapshot=REGISTRY)
    return {
        "commands": [
            {
                "name": command.name,
                "subcommand": command.subcommand or "",
                "args": command.args,
                "isPartial": command.is_partial,
                "partialDelimiter": command.partial_delimiter.name.lower()
                if command.partial_delimiter is not None
                else "",
                "precedingComment": command.preceding_comment or "",
                "expandWord": command.expand_word if command.expand_word else [],
                "range": range_dict(command.range),
            }
            for command in commands
        ]
    }


def _collect_diff_layer_payloads(
    source: str,
    *,
    dialect: str,
    layers: tuple[str, ...],
) -> dict[str, object]:
    payloads: dict[str, object] = {}
    if "ast" in layers:
        payloads["ast"] = _serialise_command_ast(source)

    if "ir" in layers or "cfg" in layers:
        compiled = run_pipeline(source, dialect=dialect)
        serialised = serialise_result(compiled)
        if "ir" in layers:
            payloads["ir"] = serialised["ir"]
        if "cfg" in layers:
            payloads["cfg"] = {
                "preSsa": serialised["cfgPreSsa"],
                "postSsa": serialised["cfgPostSsa"],
            }

    return payloads


def _compute_layer_diff(
    layer: str,
    left_payload: object,
    right_payload: object,
    *,
    left_name: str,
    right_name: str,
    context: int,
) -> tuple[bool, list[str]]:
    left_text = json.dumps(left_payload, indent=2, sort_keys=True)
    right_text = json.dumps(right_payload, indent=2, sort_keys=True)
    if left_text == right_text:
        return True, []

    diff_lines = list(
        difflib.unified_diff(
            (left_text + "\n").splitlines(keepends=True),
            (right_text + "\n").splitlines(keepends=True),
            fromfile=f"{layer}:{left_name}",
            tofile=f"{layer}:{right_name}",
            n=context,
        )
    )
    return False, diff_lines


def _run_diff(args: argparse.Namespace) -> int:
    if args.context < 0:
        raise TclCliError("--context must be >= 0")

    layers = _parse_diff_layers(args.show)
    recursive = not args.no_recursive

    left_source, left_docs = _read_diff_side(
        args.left,
        inline_sources=args.left_source,
        package_paths=args.package_path,
        recursive=recursive,
    )
    right_source, right_docs = _read_diff_side(
        args.right,
        inline_sources=args.right_source,
        package_paths=args.package_path,
        recursive=recursive,
    )

    left_payloads = _collect_diff_layer_payloads(
        left_source,
        dialect=args.dialect,
        layers=layers,
    )
    right_payloads = _collect_diff_layer_payloads(
        right_source,
        dialect=args.dialect,
        layers=layers,
    )

    layer_results: dict[str, dict[str, Any]] = {}
    has_differences = False
    for layer in layers:
        equal, diff_lines = _compute_layer_diff(
            layer,
            left_payloads[layer],
            right_payloads[layer],
            left_name=args.left,
            right_name=args.right,
            context=args.context,
        )
        has_differences |= not equal
        layer_results[layer] = {
            "equal": equal,
            "diff": [line.rstrip("\n") for line in diff_lines],
            "diffLineCount": len(diff_lines),
        }

    if args.json:
        payload = {
            "equal": not has_differences,
            "dialect": args.dialect,
            "layers": layer_results,
            "leftInput": args.left,
            "rightInput": args.right,
            "leftDocuments": left_docs,
            "rightDocuments": right_docs,
        }
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 1 if has_differences else 0

    chunks: list[str] = []
    for layer in layers:
        result = layer_results[layer]
        if bool(result["equal"]):
            chunks.append(f"{layer}: identical\n")
            continue

        chunks.append(f"=== {layer} diff ===\n")
        diff_lines = cast(list[str], result.get("diff", []))
        if diff_lines:
            for line in diff_lines:
                chunks.append(f"{line}\n")
        else:
            chunks.append("(differences detected, but no unified diff lines were produced)\n")

    _write_text_output(args.output, "".join(chunks).rstrip("\n"))
    return 1 if has_differences else 0


def _run_explore(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    source = _combine_sources(documents)

    explorer_args = [
        "--source",
        source,
        "--show",
        args.show,
        "--dialect",
        args.dialect,
        "--max-annotations",
        str(args.max_annotations),
    ]
    if args.show_optimised_source:
        explorer_args.append("--show-optimised-source")
    if args.no_source_callouts:
        explorer_args.append("--no-source-callouts")
    if args.no_colour:
        explorer_args.append("--no-colour")

    return explorer_main(explorer_args)


def _load_help_queries():
    try:
        from core.help.kcs_db import list_features, search_help
    except Exception as exc:
        raise TclCliError(
            "KCS help database is unavailable. Build it with 'make kcs-db' "
            "and rebuild the zipapp artifact."
        ) from exc
    return list_features, search_help


def _print_help_catalogue(catalogue: dict[str, list[dict[str, object]]]) -> None:
    if not catalogue:
        print("no help entries found")
        return

    for category in sorted(catalogue.keys(), key=str.lower):
        features = sorted(
            catalogue[category],
            key=lambda item: str(item.get("name", "")).lower(),
        )
        print(f"{category} ({len(features)}):")
        for feature in features:
            name = str(feature.get("name", "")).strip() or "<unnamed>"
            summary = str(feature.get("summary", "")).strip()
            if summary:
                print(f"  {name}: {summary}")
            else:
                print(f"  {name}")


def _print_help_search_results(query: str, results: list[dict[str, object]]) -> None:
    if not results:
        print(f"no KCS help matches for '{query}'", file=sys.stderr)
        return

    match_word = "match" if len(results) == 1 else "matches"
    print(f"{len(results)} {match_word} for '{query}':")
    for result in results:
        name = str(result.get("name", "")).strip() or "<unnamed>"
        category = str(result.get("category", "")).strip()
        summary = str(result.get("summary", "")).strip()
        file_name = str(result.get("file", "")).strip()

        heading = name if not category else f"{name} [{category}]"
        print(f"- {heading}")
        if summary:
            print(f"  {summary}")
        if file_name:
            print(f"  file: {file_name}")


def _dialect_help_terms(dialect: str) -> tuple[str, ...]:
    if dialect == "all":
        return ()
    return _HELP_DIALECT_TERMS.get(dialect, ())


def _help_entry_matches_dialect(entry: dict[str, object], dialect: str) -> bool:
    terms = _dialect_help_terms(dialect)
    if not terms:
        return True

    searchable_parts = (
        str(entry.get("name", "")),
        str(entry.get("summary", "")),
        str(entry.get("surface", "")),
        str(entry.get("category", "")),
        str(entry.get("how_to_use", "")),
        str(entry.get("file", "")),
    )
    searchable = " ".join(searchable_parts).lower()
    return any(term in searchable for term in terms)


def _filter_help_results_by_dialect(
    results: list[dict[str, object]],
    *,
    dialect: str,
    limit: int,
) -> list[dict[str, object]]:
    filtered = [item for item in results if _help_entry_matches_dialect(item, dialect)]
    return filtered[:limit]


def _filter_catalogue_by_dialect(
    catalogue: dict[str, list[dict[str, object]]],
    *,
    dialect: str,
) -> dict[str, list[dict[str, object]]]:
    if dialect == "all":
        return catalogue

    filtered_catalogue: dict[str, list[dict[str, object]]] = {}
    for category, features in catalogue.items():
        matched_features = []
        for feature in features:
            feature_with_category = dict(feature)
            feature_with_category.setdefault("category", category)
            if _help_entry_matches_dialect(feature_with_category, dialect):
                matched_features.append(feature)
        if matched_features:
            filtered_catalogue[category] = matched_features
    return filtered_catalogue


def _run_help(args: argparse.Namespace) -> int:
    if args.limit < 1:
        raise TclCliError("--limit must be >= 1")

    list_features, search_help = _load_help_queries()
    query = " ".join(args.query).strip()

    if not query:
        catalogue = list_features()
        catalogue = _filter_catalogue_by_dialect(catalogue, dialect=args.dialect)
        if args.json:
            print(json.dumps(catalogue, indent=2))
        else:
            _print_help_catalogue(catalogue)
        return 0 if catalogue else 1

    search_limit = args.limit if args.dialect == "all" else max(args.limit * 5, args.limit)
    results = search_help(query, limit=search_limit)
    results = _filter_help_results_by_dialect(
        results,
        dialect=args.dialect,
        limit=args.limit,
    )
    if args.json:
        print(json.dumps(results, indent=2))
    else:
        _print_help_search_results(query, results)

    if results:
        return 0

    if not args.json:
        catalogue = list_features()
        if catalogue:
            print("available sections:", file=sys.stderr)
            for category in sorted(catalogue.keys(), key=str.lower):
                print(f"  {category}", file=sys.stderr)
    return 1


# ---------------------------------------------------------------------------
# Verb catalogue — drives both the brief and full help output.
# ---------------------------------------------------------------------------

_VERB_CATALOGUE: list[tuple[str, str, str]] = [
    # (name, aliases, description)
    ("format", "fmt", "Reformat Tcl source"),
    ("opt", "optimise", "Apply optimisations and emit rewritten Tcl"),
    ("minify", "min", "Minify source code"),
    ("highlight", "hl", "Syntax-highlight source (ANSI/HTML)"),
    ("diag", "diagnostics", "Run diagnostics"),
    ("lint", "", "Run lint checks"),
    ("validate", "", "Validate source (errors only)"),
    ("symbols", "syms", "Emit symbol definitions"),
    ("diagram", "", "Extract control-flow diagram data"),
    ("callgraph", "call-graph", "Build procedure call graph"),
    ("symbolgraph", "symbol-graph", "Build symbol relationship graph"),
    ("dataflow", "dataflow-graph", "Build taint/effect data-flow graph"),
    ("event-order", "eventorder", "Show iRules events in firing order"),
    ("event-info", "eventinfo", "Look up iRules event metadata"),
    ("command-info", "cmd-info", "Look up command registry metadata"),
    ("convert", "", "Detect legacy patterns for modernisation"),
    ("dis", "asm", "Disassemble compiled bytecode"),
    ("compwasm", "wasm", "Compile to WebAssembly binary"),
    ("diff", "", "Diff two sources via AST/IR/CFG"),
    ("explore", "", "Run compiler-explorer views"),
    ("help", "docs", "Search bundled KCS help docs"),
]


class _BriefHelpAction(argparse.Action):
    """Print a compact help overview and exit."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        prog = parser.prog
        lines = [
            f"{prog} <verb> [options] [inputs...]\n",
            "Verbs:",
        ]
        for name, aliases, desc in _VERB_CATALOGUE:
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
        # Print main parser help.
        parser.print_help()
        print()

        # Print each subparser's help.
        for action in parser._subparsers._actions:  # noqa: SLF001
            if not isinstance(action, argparse._SubParsersAction):  # noqa: SLF001
                continue
            for name, subparser in action.choices.items():
                # Skip aliases — only print each verb once.
                if name != subparser.prog.split()[-1]:
                    continue
                print(f"\n{'=' * 60}")
                print(f"  {name}")
                print(f"{'=' * 60}\n")
                subparser.print_help()
        parser.exit()


def parse_args(
    argv: list[str],
    *,
    prog_name: str = "tcl",
    default_dialect: str = "tcl8.6",
) -> argparse.Namespace:
    help_default_dialect = "f5-irules" if default_dialect == "f5-irules" else "all"

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

    opt_p = sub.add_parser(
        "opt",
        aliases=["optimise", "optimize"],
        help="Optimise source and output rewritten Tcl.",
    )
    _add_input_arguments(opt_p, include_output=True, default_dialect=default_dialect)
    _add_colour_arguments(opt_p)
    _add_toggle_arguments(opt_p, kind="optimisation")
    opt_p.set_defaults(handler=_run_opt)

    diag_p = sub.add_parser(
        "diag",
        aliases=["diagnostics"],
        help="Run diagnostics across all resolved inputs.",
    )
    _add_input_arguments(diag_p, default_dialect=default_dialect)
    diag_p.add_argument(
        "--json",
        action="store_true",
        help="Emit diagnostics as JSON.",
    )
    _add_toggle_arguments(diag_p, kind="diagnostic")
    diag_p.set_defaults(handler=_run_diag)

    lint_p = sub.add_parser(
        "lint",
        help="Run lint diagnostics across all resolved inputs.",
    )
    _add_input_arguments(lint_p, default_dialect=default_dialect)
    lint_p.add_argument(
        "--json",
        action="store_true",
        help="Emit diagnostics as JSON.",
    )
    _add_toggle_arguments(lint_p, kind="diagnostic")
    lint_p.set_defaults(handler=_run_diag)

    val_p = sub.add_parser(
        "validate",
        help="Validate source (error-level diagnostics only).",
    )
    _add_input_arguments(val_p, default_dialect=default_dialect)
    val_p.add_argument(
        "--json",
        action="store_true",
        help="Emit validation results as JSON.",
    )
    _add_toggle_arguments(val_p, kind="diagnostic")
    val_p.set_defaults(handler=_run_validate)

    format_p = sub.add_parser(
        "format",
        aliases=["fmt"],
        help="Format source and emit rewritten Tcl.",
        description="Format source and emit rewritten Tcl.",
        epilog=(
            "Examples:\n"
            f"  {prog_name} format script.tcl\n"
            f"  {prog_name} format src/ --dialect f5-irules -o formatted.tcl\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    _add_input_arguments(format_p, include_output=True, default_dialect=default_dialect)
    _add_colour_arguments(format_p)
    _add_formatter_arguments(format_p)
    format_p.set_defaults(handler=_run_format)

    minify_p = sub.add_parser(
        "minify",
        aliases=["min"],
        help="Minify source: strip comments, collapse whitespace, join commands.",
        description="Minify Tcl source code: strip comments, collapse whitespace, join commands with semicolons.",
        epilog=(
            "Examples:\n"
            f"  {prog_name} minify script.tcl\n"
            f"  {prog_name} minify script.tcl -o minified.tcl\n"
            f"  {prog_name} minify --compact script.tcl --symbol-map symbols.txt\n"
            f"  {prog_name} minify --aggressive script.tcl -o tiny.tcl\n"
            f"  {prog_name} minify src/ --dialect f5-irules\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    _add_input_arguments(minify_p, include_output=True, default_dialect=default_dialect)
    minify_p.add_argument(
        "--compact",
        action="store_true",
        help="Compact variable and proc names to short identifiers.",
    )
    minify_p.add_argument(
        "--symbol-map",
        metavar="FILE",
        default=None,
        help="Write symbol map (original -> compacted names) to FILE.",
    )
    minify_p.add_argument(
        "--aggressive",
        action="store_true",
        help="Maximum compression: run all optimiser passes, then compact names and minify.",
    )
    _add_colour_arguments(minify_p)
    minify_p.set_defaults(handler=_run_minify)

    unminify_err_p = sub.add_parser(
        "unminify-error",
        aliases=["umerr"],
        help="Translate a minified-code error message back to original names.",
        description=(
            "Translate Tcl or iRule error messages produced by minified code "
            "back to the original variable, proc, and command names using a "
            "saved symbol map."
        ),
        epilog=(
            "Examples:\n"
            f"  {prog_name} unminify-error --symbol-map map.txt --error 'can't read \"a\": no such variable'\n"
            f"  {prog_name} unminify-error --symbol-map map.txt --error-file /var/log/ltm\n"
            f"  {prog_name} unminify-error --symbol-map map.txt --minified min.tcl --original src.tcl --error-file err.log\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    unminify_err_p.add_argument(
        "--symbol-map",
        metavar="FILE",
        required=True,
        help="Path to the symbol map file produced by minify --symbol-map.",
    )
    unminify_err_p.add_argument(
        "--error",
        "-e",
        metavar="TEXT",
        help="Error message text to translate (inline).",
    )
    unminify_err_p.add_argument(
        "--error-file",
        metavar="FILE",
        help="File containing error messages to translate ('-' for stdin).",
    )
    unminify_err_p.add_argument(
        "--minified",
        metavar="FILE",
        help="The minified source file (for line-number remapping).",
    )
    unminify_err_p.add_argument(
        "--original",
        metavar="FILE",
        help="The original source file (for line-number remapping).",
    )
    unminify_err_p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    unminify_err_p.set_defaults(handler=_run_unminify_error)

    symbols_p = sub.add_parser(
        "symbols",
        aliases=["syms"],
        help="Emit symbol definitions for the resolved input.",
    )
    _add_input_arguments(symbols_p, include_output=True, default_dialect=default_dialect)
    symbols_p.add_argument(
        "--json",
        action="store_true",
        help="Emit symbol data as JSON.",
    )
    symbols_p.set_defaults(handler=_run_symbols)

    diagram_p = sub.add_parser(
        "diagram",
        help="Extract control-flow diagram data from compiler IR.",
    )
    _add_input_arguments(diagram_p, include_output=True, default_dialect=default_dialect)
    diagram_p.add_argument(
        "--json",
        action="store_true",
        help="Emit diagram data as JSON.",
    )
    diagram_p.set_defaults(handler=_run_diagram)

    callgraph_p = sub.add_parser(
        "callgraph",
        aliases=["call-graph"],
        help="Build call graph data for resolved source.",
    )
    _add_input_arguments(callgraph_p, include_output=True, default_dialect=default_dialect)
    callgraph_p.add_argument(
        "--json",
        action="store_true",
        help="Emit call graph data as JSON.",
    )
    callgraph_p.set_defaults(handler=_run_callgraph)

    symbolgraph_p = sub.add_parser(
        "symbolgraph",
        aliases=["symbol-graph"],
        help="Build symbol relationship graph data.",
    )
    _add_input_arguments(symbolgraph_p, include_output=True, default_dialect=default_dialect)
    symbolgraph_p.add_argument(
        "--json",
        action="store_true",
        help="Emit symbol graph data as JSON.",
    )
    symbolgraph_p.set_defaults(handler=_run_symbolgraph)

    dataflow_p = sub.add_parser(
        "dataflow",
        aliases=["dataflow-graph"],
        help="Build taint/effect data-flow graph data.",
    )
    _add_input_arguments(dataflow_p, include_output=True, default_dialect=default_dialect)
    dataflow_p.add_argument(
        "--json",
        action="store_true",
        help="Emit data-flow graph data as JSON.",
    )
    dataflow_p.set_defaults(handler=_run_dataflow)

    event_order_p = sub.add_parser(
        "event-order",
        aliases=["eventorder"],
        help="Show iRules events in canonical firing order.",
    )
    _add_input_arguments(event_order_p, include_output=True, default_dialect=default_dialect)
    event_order_p.add_argument(
        "--json",
        action="store_true",
        help="Emit event ordering as JSON.",
    )
    event_order_p.set_defaults(handler=_run_event_order)

    event_info_p = sub.add_parser(
        "event-info",
        aliases=["eventinfo"],
        help="Look up iRules event metadata and valid commands.",
    )
    event_info_p.add_argument(
        "event",
        help="iRules event name (for example: HTTP_REQUEST).",
    )
    event_info_p.add_argument(
        "--json",
        action="store_true",
        help="Emit event metadata as JSON.",
    )
    event_info_p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    event_info_p.set_defaults(handler=_run_event_info)

    command_info_p = sub.add_parser(
        "command-info",
        aliases=["commandinfo", "cmd-info"],
        help="Look up command registry metadata.",
        description="Look up command registry metadata.",
    )
    command_info_p.add_argument(
        "command",
        help="Command name to query (for example: HTTP::uri or string).",
    )
    command_info_p.add_argument(
        "--dialect",
        choices=AVAILABLE_DIALECTS,
        default=default_dialect,
        help=(f"Dialect profile for command metadata lookup (default: {default_dialect})."),
    )
    command_info_p.add_argument(
        "--json",
        action="store_true",
        help="Emit command metadata as JSON.",
    )
    command_info_p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    command_info_p.set_defaults(handler=_run_command_info)

    convert_p = sub.add_parser(
        "convert",
        help="Detect legacy patterns eligible for modernisation.",
    )
    _add_input_arguments(convert_p, include_output=True, default_dialect=default_dialect)
    convert_p.add_argument(
        "--json",
        action="store_true",
        help="Emit convert findings as JSON.",
    )
    convert_p.set_defaults(handler=_run_convert)

    dis_p = sub.add_parser(
        "dis",
        aliases=["asm", "disassemble"],
        help="Compile and emit bytecode disassembly.",
    )
    _add_input_arguments(dis_p, include_output=True, default_dialect=default_dialect)
    dis_p.add_argument(
        "--optimise",
        action="store_true",
        help="Enable optimiser path before disassembly.",
    )
    dis_p.set_defaults(handler=_run_dis)

    wasm_p = sub.add_parser(
        "compwasm",
        aliases=["wasm"],
        help="Compile source to WebAssembly binary.",
    )
    _add_input_arguments(wasm_p, include_output=True, default_dialect=default_dialect)
    wasm_p.add_argument(
        "--optimise",
        "-O",
        action="store_true",
        help="Enable WebAssembly optimisation passes.",
    )
    wasm_p.add_argument(
        "--wat-output",
        default="",
        help="Optional path for WAT text output.",
    )
    wasm_p.set_defaults(handler=_run_compwasm, output="out.wasm")

    highlight_p = sub.add_parser(
        "highlight",
        aliases=["hl"],
        help="Emit syntax-highlighted source output.",
        description="Emit syntax-highlighted source output.",
        epilog=(
            "Examples:\n"
            f"  {prog_name} highlight script.tcl --colour\n"
            f"  {prog_name} highlight script.tcl --format html -o out.html\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    _add_input_arguments(highlight_p, include_output=True, default_dialect=default_dialect)
    highlight_p.add_argument(
        "--format",
        choices=("ansi", "html"),
        default="ansi",
        help="Output format (default: ansi).",
    )
    _add_colour_arguments(highlight_p)
    highlight_p.set_defaults(handler=_run_highlight)

    diff_p = sub.add_parser(
        "diff",
        help="Diff two sources using AST, IR, and CFG representations.",
        description="Diff two sources using AST, IR, and CFG representations.",
        epilog=(
            "Examples:\n"
            f"  {prog_name} diff old.irule new.irule --show ast,ir,cfg\n"
            f"  {prog_name} diff old.irule new.irule --show ir --context 5\n"
            f"  {prog_name} diff old.irule new.irule --json\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    diff_p.add_argument(
        "left",
        help="Left input (file, directory, or package name).",
    )
    diff_p.add_argument(
        "right",
        help="Right input (file, directory, or package name).",
    )
    diff_p.add_argument(
        "--left-source",
        action="append",
        default=[],
        help="Inline source chunk to append to the left input side.",
    )
    diff_p.add_argument(
        "--right-source",
        action="append",
        default=[],
        help="Inline source chunk to append to the right input side.",
    )
    diff_p.add_argument(
        "--package-path",
        action="append",
        default=[],
        help="Additional directory to scan for pkgIndex.tcl package metadata.",
    )
    diff_p.add_argument(
        "--no-recursive",
        action="store_true",
        help="Do not recurse when an input is a directory.",
    )
    diff_p.add_argument(
        "--dialect",
        choices=AVAILABLE_DIALECTS,
        default=default_dialect,
        help=(f"Dialect profile for IR/CFG lowering during diff (default: {default_dialect})."),
    )
    diff_p.add_argument(
        "--show",
        default="ast,ir,cfg",
        help="Comma-separated diff layers: ast,ir,cfg,all (default: ast,ir,cfg).",
    )
    diff_p.add_argument(
        "--context",
        type=int,
        default=3,
        help="Unified diff context lines (default: 3).",
    )
    diff_p.add_argument(
        "--json",
        action="store_true",
        help="Emit diff results as JSON.",
    )
    diff_p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    diff_p.set_defaults(handler=_run_diff)

    explore_p = sub.add_parser(
        "explore",
        help="Run compiler-explorer views on aggregated input.",
    )
    _add_input_arguments(explore_p, default_dialect=default_dialect)
    explore_p.add_argument(
        "--show",
        default="all",
        help="Compiler explorer views to show (default: all).",
    )
    explore_p.add_argument(
        "--show-optimised-source",
        action="store_true",
        help="Include line-numbered optimised source output.",
    )
    explore_p.add_argument(
        "--no-source-callouts",
        action="store_true",
        help="Disable source callout rendering.",
    )
    explore_p.add_argument(
        "--max-annotations",
        type=int,
        default=80,
        help="Maximum source callouts to render (-1 for unlimited).",
    )
    explore_p.add_argument(
        "--no-colour",
        "--no-color",
        dest="no_colour",
        action="store_true",
        help="Disable ANSI colour output.",
    )
    explore_p.set_defaults(handler=_run_explore)

    help_p = sub.add_parser(
        "help",
        aliases=["docs"],
        help="Search KCS help docs from the bundled SQLite index.",
        description="Search KCS help docs from the bundled SQLite index.",
        epilog=(
            "Examples:\n"
            f"  {prog_name} help taint\n"
            f"  {prog_name} help event --dialect f5-irules\n"
            f"  {prog_name} help --dialect tcl8.6 --json\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    help_p.add_argument(
        "query",
        nargs="*",
        help="Search terms (omit to list available help sections).",
    )
    help_p.add_argument(
        "--dialect",
        choices=("all", *AVAILABLE_DIALECTS),
        default=help_default_dialect,
        help=(f"Filter help matches by dialect context (default: {help_default_dialect})."),
    )
    help_p.add_argument(
        "--limit",
        type=int,
        default=20,
        help="Maximum number of help search matches (default: 20).",
    )
    help_p.add_argument(
        "--json",
        action="store_true",
        help="Emit help results as JSON.",
    )
    help_p.set_defaults(handler=_run_help)

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
    default_dialect = _default_dialect_for_prog(selected_prog_name)
    cli_config = _load_config()
    args = parse_args(
        parsed_argv,
        prog_name=selected_prog_name,
        default_dialect=default_dialect,
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
