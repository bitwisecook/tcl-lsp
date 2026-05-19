#!/usr/bin/env python3
"""Filter README.md for a single-editor distribution.

Produces a per-editor README by keeping only sections relevant to the
target editor and stripping references to other editors.  Uses a simple
line-by-line markdown parser that understands headings and fenced code
blocks (so ``#`` inside code is never mistaken for a heading).

**Heading conventions** (the source README must follow these):

1. Editor subsections under ``## Editor support`` are named
   ``### EditorName`` (e.g. ``### VS Code``, ``### Neovim``).
   The filter keeps only the target editor's subsection.

2. Sections tagged with ``(EditorName ...)`` at the end of the heading
   (e.g. ``### Compiler explorer (VS Code panel)``) are kept only for
   the matching editor, with the parenthetical removed from the heading.

3. A ``### All editors`` section holds content that appears only in the
   multi-editor README (e.g. the comparison table).  It is stripped for
   all single-editor builds.

4. Lines containing ``<!-- editors:Name1,Name2 -->`` are kept only when
   the target editor appears in the comma-separated list.  This handles
   rows inside tables or code blocks that cannot use headings.

Usage::

    python scripts/install/filter_readme.py --editor "VS Code" README.md -o build/README.md
    python scripts/install/filter_readme.py --editor "VS Code" README.md  # stdout
"""

from __future__ import annotations

import argparse
import re
import sys

# Editors whose subsections the filter recognises under ``## Editor support``.
KNOWN_EDITORS = frozenset(
    {
        "VS Code",
        "Neovim",
        "Zed",
        "Emacs",
        "Helix",
        "Sublime Text",
        "JetBrains",
    }
)

# Heading regex — captures level (##/###/####) and title text.
_HEADING_RE = re.compile(r"^(#{2,4})\s+(.+)$")

# Fenced code block opener/closer — ````` or ~~~, optionally with a language tag.
_FENCE_RE = re.compile(r"^(`{3,}|~{3,})")

# Parenthetical editor suffix — e.g. "(VS Code panel)", "(VS Code + GitHub Copilot)".
_PAREN_EDITOR_RE = re.compile(
    r"\((" + "|".join(re.escape(e) for e in KNOWN_EDITORS) + r")[^)]*\)\s*$"
)

# Inline editor marker — ``<!-- editors:VS Code,Neovim -->``
_INLINE_EDITOR_RE = re.compile(r"<!--\s*editors:\s*([^>]+?)\s*-->")


def filter_readme(text: str, editor: str) -> str:
    """Produce a single-editor README from the multi-editor source."""
    lines = text.split("\n")
    out: list[str] = []

    # State
    skip_until_level: int | None = None  # skip until a heading at this level or higher
    in_editor_support = False  # inside ## Editor support
    in_fence = False  # inside a fenced code block
    fence_marker: str = ""  # the full opening fence string (to match the closer)

    for line in lines:
        # ── Track fenced code blocks ─────────────────────────────────
        fence_match = _FENCE_RE.match(line)
        if fence_match:
            matched_fence = fence_match.group(1)
            if not in_fence:
                in_fence = True
                fence_marker = matched_fence
            elif matched_fence[0] == fence_marker[0] and len(matched_fence) >= len(fence_marker):
                in_fence = False
                fence_marker = ""

        # Inside a fenced code block, skip heading/section logic but
        # still respect inline editor markers and skip state.
        if in_fence and not fence_match:
            if skip_until_level is not None:
                continue
            # Check inline editor markers even inside code blocks.
            inline = _INLINE_EDITOR_RE.search(line)
            if inline:
                allowed = {e.strip() for e in inline.group(1).split(",")}
                if editor not in allowed:
                    continue
                line = _INLINE_EDITOR_RE.sub("", line).rstrip()
            out.append(line)
            continue

        # ── Handle headings (only outside code blocks) ───────────────
        heading = _HEADING_RE.match(line) if not in_fence else None

        if heading:
            level = len(heading.group(1))
            title = heading.group(2).strip()

            # If we were skipping, check whether this heading ends the skip.
            if skip_until_level is not None:
                if level <= skip_until_level:
                    skip_until_level = None
                else:
                    continue  # still inside skipped section

            # Track whether we are inside ## Editor support.
            if level == 2:
                in_editor_support = title == "Editor support"

            # Rule 3: ### All editors under ## Editor support — strip entirely.
            if in_editor_support and level == 3 and title == "All editors":
                skip_until_level = level
                continue

            # Rule 1: Editor subsections under ## Editor support.
            if in_editor_support and level == 3 and title in KNOWN_EDITORS:
                if title != editor:
                    skip_until_level = level
                    continue
                out.append(line)
                continue

            # Rule 2: Parenthetical editor qualifiers.
            paren = _PAREN_EDITOR_RE.search(title)
            if paren:
                paren_editor = paren.group(1)
                if paren_editor != editor:
                    skip_until_level = level
                    continue
                # Strip the parenthetical for the matching editor.
                clean_title = title[: paren.start()].rstrip()
                out.append(f"{heading.group(1)} {clean_title}")
                continue

            out.append(line)
            continue

        # ── Non-heading lines ────────────────────────────────────────

        # If we are inside a skipped section, drop the line.
        if skip_until_level is not None:
            continue

        # Rule 4: Inline editor markers.
        inline = _INLINE_EDITOR_RE.search(line)
        if inline:
            allowed = {e.strip() for e in inline.group(1).split(",")}
            if editor not in allowed:
                continue
            # Strip the marker comment from the kept line.
            line = _INLINE_EDITOR_RE.sub("", line).rstrip()

        out.append(line)

    result = "\n".join(out)

    # Collapse runs of 3+ blank lines to 2 (single visual gap).
    result = re.sub(r"\n{3,}", "\n\n", result)

    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", help="Input README.md path")
    parser.add_argument(
        "--editor",
        default="VS Code",
        choices=sorted(KNOWN_EDITORS),
        help=(
            "Target editor name (default: 'VS Code'). "
            f"Valid values: {', '.join(sorted(KNOWN_EDITORS))}"
        ),
    )
    parser.add_argument("-o", "--output", help="Output path (default: stdout)")
    args = parser.parse_args()

    with open(args.input, encoding="utf-8") as f:
        text = f.read()

    result = filter_readme(text, args.editor)

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(result)
    else:
        sys.stdout.write(result)


if __name__ == "__main__":
    main()
