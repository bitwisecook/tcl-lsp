"""Shared CLI utilities used by all verb modules.

Covers: input resolution, output writing, config resolution helpers,
argparse argument groups, diagnostic formatting, and syntax highlighting.
"""

from __future__ import annotations

import argparse
import html
import os
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from analyser.packages import PackageResolver
from analyser.semantic_model import Diagnostic
from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.lexer import TclLexer
from compiler.parsing.tokens import TokenType
from compiler.registry import REGISTRY
from tooling.formatter.config import FormatterConfig

from ..pipeline import AVAILABLE_DIALECTS

# ---------------------------------------------------------------------------
# Source file detection
# ---------------------------------------------------------------------------

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

# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------


@dataclass(slots=True, frozen=True)
class InputDocument:
    """Resolved input document used by command verbs."""

    label: str
    source: str
    path: Path | None = None


class TclCliError(ValueError):
    """Raised when command-line input cannot be resolved."""


# ---------------------------------------------------------------------------
# Input resolution
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# Output helpers
# ---------------------------------------------------------------------------


def _write_text_output(output_path: str, text: str) -> None:
    if output_path == "-":
        sys.stdout.write(text)
        if text and not text.endswith("\n"):
            sys.stdout.write("\n")
        return
    Path(output_path).write_text(text, encoding="utf-8")


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


# ---------------------------------------------------------------------------
# Diagnostic formatting
# ---------------------------------------------------------------------------


def _format_diagnostic_line(document: InputDocument, diagnostic: Diagnostic) -> str:
    line = diagnostic.range.start.line + 1
    column = diagnostic.range.start.character + 1
    severity = diagnostic.severity.name.lower()
    code = diagnostic.code or "-"
    return f"{document.label}:{line}:{column}: {severity:<7} {code:<8} {diagnostic.message}"


# ---------------------------------------------------------------------------
# Config resolution helpers
# ---------------------------------------------------------------------------


def _parse_code_set(raw: str | None) -> set[str]:
    """Parse a comma-separated list of codes into a set."""
    if not raw:
        return set()
    return {c.strip().upper() for c in raw.split(",") if c.strip()}


def _resolve_disabled_diagnostics(args: argparse.Namespace) -> set[str]:
    """Build the set of diagnostic codes to suppress."""
    cfg = getattr(args, "cli_config", None)
    disabled = set(cfg.disabled_diagnostics) if cfg else set()
    disabled |= _parse_code_set(getattr(args, "disable", None))
    disabled -= _parse_code_set(getattr(args, "enable", None))
    return disabled


def _resolve_disabled_optimisations(args: argparse.Namespace) -> tuple[set[str], bool, int]:
    """Build the set of optimisation codes to suppress and multi-pass params.

    Returns ``(disabled_codes, multi_pass, max_iterations)``.
    """
    from shared.optimisation_profiles import (
        DEFAULT_ACTION_PROFILE,
        profile_from_name,
        profile_spec,
        profile_to_disabled,
    )

    cfg = getattr(args, "cli_config", None)
    profile_name = getattr(args, "profile", None)
    if profile_name:
        try:
            profile = profile_from_name(profile_name)
        except ValueError:
            profile = DEFAULT_ACTION_PROFILE
    elif cfg and cfg.optimiser_profile:
        try:
            profile = profile_from_name(cfg.optimiser_profile)
        except ValueError:
            profile = DEFAULT_ACTION_PROFILE
    else:
        profile = DEFAULT_ACTION_PROFILE

    spec = profile_spec(profile)
    disabled = set(profile_to_disabled(profile))
    if cfg:
        disabled = set(cfg.disabled_optimisations)
        if profile_name:
            disabled = set(profile_to_disabled(profile))
    disabled |= _parse_code_set(getattr(args, "disable", None))
    disabled -= _parse_code_set(getattr(args, "enable", None))
    return disabled, spec.multi_pass, spec.max_iterations


def _resolve_formatter_config(args: argparse.Namespace) -> FormatterConfig:
    """Build a FormatterConfig from config file + CLI flag overrides."""
    cfg = getattr(args, "cli_config", None)
    base = cfg.formatter if cfg else FormatterConfig()

    from tooling.formatter.config import IndentStyle

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
    if getattr(args, "force_colour", False):
        return True
    if getattr(args, "no_colour", False):
        return False
    cfg = getattr(args, "cli_config", None)
    if cfg is not None:
        if cfg.colour_mode == "always":
            return True
        if cfg.colour_mode == "never":
            return False
    return getattr(args, "output", "-") == "-" and sys.stdout.isatty()


def _resolve_tab_width(args: argparse.Namespace) -> int:
    """Return the effective tab expansion width."""
    cli_val = getattr(args, "tabs", None)
    if cli_val is not None:
        return cli_val
    cfg = getattr(args, "cli_config", None)
    if cfg is not None:
        return cfg.tab_width
    return 4


# ---------------------------------------------------------------------------
# Argument parser helpers
# ---------------------------------------------------------------------------


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


def _add_input_arguments(
    parser: argparse.ArgumentParser,
    *,
    include_output: bool = False,
    default_dialect: str,
) -> None:
    inputs = parser.add_argument(
        "inputs",
        nargs="*",
        help="Input files, directories, or package names.",
    )
    _attach_tcl_file_completer(inputs)

    parser.add_argument(
        "--source",
        action="append",
        default=[],
        help="Inline Tcl source text (can be repeated).",
    )
    pkg_path = parser.add_argument(
        "--package-path",
        action="append",
        default=[],
        help="Additional directory to scan for pkgIndex.tcl package metadata.",
    )
    _attach_directory_completer(pkg_path)
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
        out = parser.add_argument(
            "--output",
            "-o",
            default="-",
            help="Output path ('-' for stdout).",
        )
        _attach_file_completer(out)


def _attach_tcl_file_completer(action: argparse.Action) -> None:
    """Attach a Tcl/iRule file-pattern completer for argcomplete.

    Picks up ``.tcl`` / ``.tm`` / ``.irul`` / ``.iapp`` / ``.impl`` so the
    shell narrows positional completion to source files instead of every
    file in the cwd.
    """
    try:
        from argcomplete.completers import FilesCompleter
    except ImportError:
        return
    action.completer = FilesCompleter(  # type: ignore[attr-defined]  # ty: ignore[unresolved-attribute]  # argcomplete extends argparse.Action at runtime.
        allowednames=("tcl", "tm", "tk", "itcl", "irul", "irule", "iapp", "iappimpl", "impl"),
        directories=True,
    )


def _attach_file_completer(action: argparse.Action) -> None:
    """Attach a plain file completer (any file)."""
    try:
        from argcomplete.completers import FilesCompleter
    except ImportError:
        return
    action.completer = FilesCompleter()  # type: ignore[attr-defined]  # ty: ignore[unresolved-attribute]  # argcomplete extends argparse.Action at runtime.


def _attach_directory_completer(action: argparse.Action) -> None:
    """Attach a directory-only completer."""
    try:
        from argcomplete.completers import DirectoriesCompleter
    except ImportError:
        return
    action.completer = DirectoriesCompleter()  # type: ignore[attr-defined]  # ty: ignore[unresolved-attribute]  # argcomplete extends argparse.Action at runtime.


# ---------------------------------------------------------------------------
# Syntax highlighting implementation
# ---------------------------------------------------------------------------

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

        if token.type is TokenType.STR and _is_body_token(token_text):
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
