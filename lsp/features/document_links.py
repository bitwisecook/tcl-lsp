"""Document link provider -- make 'source' paths and 'package require' clickable."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, Range
from core.common.lsp import to_lsp_range
from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import TokenType


def _collect_source_links(source: str) -> list[types.DocumentLink]:
    """Find 'source filename' commands and create links for the filename."""
    links: list[types.DocumentLink] = []
    for cmd in segment_commands(source):
        if not cmd.texts:
            continue
        if cmd.texts[0] == "source" and len(cmd.argv) >= 2:
            path_tok = cmd.argv[1]
            if path_tok.type in (TokenType.ESC, TokenType.STR):
                path_text = path_tok.text
                # Skip variable references and command substitutions
                if "$" in path_text or "[" in path_text:
                    continue
                links.append(
                    types.DocumentLink(
                        range=to_lsp_range(Range(start=path_tok.start, end=path_tok.end)),
                        tooltip=f"Open {path_text}",
                    )
                )
    return links


def _collect_package_require_links(
    analysis: AnalysisResult,
) -> list[types.DocumentLink]:
    """Create links for 'package require' statements."""
    links: list[types.DocumentLink] = []
    for pkg in analysis.package_requires:
        tooltip = f"package require {pkg.name}"
        if pkg.version:
            tooltip += f" {pkg.version}"
        links.append(
            types.DocumentLink(
                range=to_lsp_range(pkg.range),
                tooltip=tooltip,
            )
        )
    return links


def get_document_links(
    source: str,
    analysis: AnalysisResult | None = None,
) -> list[types.DocumentLink]:
    """Return document links for source commands and package requires."""
    if analysis is None:
        analysis = analyse(source)

    links: list[types.DocumentLink] = []
    links.extend(_collect_source_links(source))
    links.extend(_collect_package_require_links(analysis))
    return links
