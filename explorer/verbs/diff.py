"""Diff verb: compare two Tcl/iRules sources via AST, IR, and CFG layers."""

from __future__ import annotations

import argparse
import difflib
import json
from typing import Any, cast

from compiler.parsing.command_segmenter import segment_commands
from compiler.registry import REGISTRY

from ..formatters import range_dict
from ..pipeline import AVAILABLE_DIALECTS, run_pipeline
from ..serialise import serialise_result
from ._registry import verb
from ._utils import (
    TclCliError,
    _combine_sources,
    _read_input_documents,
    _write_text_output,
)

_DIFF_LAYERS = ("ast", "ir", "cfg")


@verb(
    "diff",
    help="Diff two sources using AST, IR, and CFG representations.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_diff(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    p.description = (
        "Compare *left* and *right* sources at the AST, IR, and CFG layers\n"
        "and emit a unified diff.  Distinct from `git diff` / textual diff:\n"
        "two sources that differ only in whitespace, comments, or variable\n"
        "names produce no AST-level diff, while two sources that look\n"
        "identical but optimise differently surface in the CFG diff.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} diff old.irule new.irule --show ast,ir,cfg\n"
        f"  {prog_name} diff old.irule new.irule --show ir --context 5\n"
        f"  {prog_name} diff old.irule new.irule --json\n"
    )
    p.add_argument(
        "left",
        help="Left input (file, directory, or package name).",
    )
    p.add_argument(
        "right",
        help="Right input (file, directory, or package name).",
    )
    p.add_argument(
        "--left-source",
        action="append",
        default=[],
        help="Inline source chunk to append to the left input side.",
    )
    p.add_argument(
        "--right-source",
        action="append",
        default=[],
        help="Inline source chunk to append to the right input side.",
    )
    p.add_argument(
        "--package-path",
        action="append",
        default=[],
        help="Additional directory to scan for pkgIndex.tcl package metadata.",
    )
    p.add_argument(
        "--no-recursive",
        action="store_true",
        help="Do not recurse when an input is a directory.",
    )
    p.add_argument(
        "--dialect",
        choices=AVAILABLE_DIALECTS,
        default=default_dialect,
        help=(f"Dialect profile for IR/CFG lowering during diff (default: {default_dialect})."),
    )
    p.add_argument(
        "--show",
        default="ast,ir,cfg",
        help="Comma-separated diff layers: ast,ir,cfg,all (default: ast,ir,cfg).",
    )
    p.add_argument(
        "--context",
        type=int,
        default=3,
        help="Unified diff context lines (default: 3).",
    )
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit diff results as JSON.",
    )
    p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    p.set_defaults(handler=_run_diff)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# Handler
# ---------------------------------------------------------------------------


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
