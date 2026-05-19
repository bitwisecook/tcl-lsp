"""Tests for scripts/filter_readme.py — heading-based README filtering."""

from __future__ import annotations

import sys
from pathlib import Path
from textwrap import dedent

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.install.filter_readme import KNOWN_EDITORS, filter_readme  # noqa: E402

# ---------------------------------------------------------------------------
# Rule 1: Editor subsections under ## Editor support
# ---------------------------------------------------------------------------

EDITOR_SECTIONS = dedent("""\
    ## Editor support

    Some intro text.

    ### All editors

    | Editor | Info |
    |--------|------|
    | VS Code | ext |
    | Neovim | lua |

    ### VS Code

    VS Code content here.

    ### Neovim

    Neovim content here.

    ### Emacs

    Emacs content here.

    ## Features

    Feature text.
""")


def test_keeps_target_editor_section():
    result = filter_readme(EDITOR_SECTIONS, "VS Code")
    assert "VS Code content here." in result
    assert "Neovim content here." not in result
    assert "Emacs content here." not in result


def test_keeps_different_target():
    result = filter_readme(EDITOR_SECTIONS, "Neovim")
    assert "Neovim content here." in result
    assert "VS Code content here." not in result
    assert "Emacs content here." not in result


def test_strips_all_editors_section():
    result = filter_readme(EDITOR_SECTIONS, "VS Code")
    assert "### All editors" not in result
    assert "| Editor | Info |" not in result


def test_preserves_content_after_editor_sections():
    result = filter_readme(EDITOR_SECTIONS, "VS Code")
    assert "## Features" in result
    assert "Feature text." in result


# ---------------------------------------------------------------------------
# Rule 2: Parenthetical editor qualifiers
# ---------------------------------------------------------------------------

PAREN_SECTIONS = dedent("""\
    ## Features

    ### Compiler explorer (VS Code panel)

    Explorer content.

    ### Some feature (JetBrains IDE)

    JetBrains content.

    ### Generic feature

    Generic content.
""")


def test_keeps_matching_paren_and_strips_suffix():
    result = filter_readme(PAREN_SECTIONS, "VS Code")
    assert "### Compiler explorer" in result
    assert "(VS Code panel)" not in result
    assert "Explorer content." in result


def test_strips_non_matching_paren_section():
    result = filter_readme(PAREN_SECTIONS, "VS Code")
    assert "JetBrains content." not in result
    assert "Some feature" not in result


def test_preserves_generic_sections():
    result = filter_readme(PAREN_SECTIONS, "VS Code")
    assert "### Generic feature" in result
    assert "Generic content." in result


# ---------------------------------------------------------------------------
# Rule 3: ### All editors scoped to ## Editor support
# ---------------------------------------------------------------------------

ALL_EDITORS_OUTSIDE = dedent("""\
    ## Other section

    ### All editors

    This should NOT be stripped — it's not under Editor support.

    ## Editor support

    ### All editors

    This SHOULD be stripped.

    ### VS Code

    Kept.

    ## Another section

    More content.
""")


def test_all_editors_only_stripped_under_editor_support():
    result = filter_readme(ALL_EDITORS_OUTSIDE, "VS Code")
    # The one outside ## Editor support is kept.
    assert "This should NOT be stripped" in result
    # The one inside is stripped.
    assert "This SHOULD be stripped" not in result


# ---------------------------------------------------------------------------
# Rule 4: Inline editor markers
# ---------------------------------------------------------------------------

INLINE_MARKERS = dedent("""\
    ## Build targets

    | Target | Description |
    |--------|-------------|
    | `make vsix` | Build VSIX |
    | `make jetbrains` | Build JetBrains | <!-- editors:JetBrains -->
    | `make sublime` | Build Sublime | <!-- editors:Sublime Text -->
    | `make zed` | Build Zed | <!-- editors:Zed -->

    Done.
""")


def test_inline_markers_strip_non_matching():
    result = filter_readme(INLINE_MARKERS, "VS Code")
    assert "make vsix" in result
    assert "make jetbrains" not in result
    assert "make sublime" not in result
    assert "make zed" not in result
    assert "Done." in result


def test_inline_markers_keep_matching():
    result = filter_readme(INLINE_MARKERS, "JetBrains")
    assert "make jetbrains" in result
    assert "make sublime" not in result
    # The marker comment itself should be removed from kept lines.
    assert "<!-- editors:" not in result


# ---------------------------------------------------------------------------
# Fenced code blocks — headings inside code are not parsed
# ---------------------------------------------------------------------------

FENCED_CODE = dedent("""\
    ## Features

    ```sh
    ### This is not a heading
    # Neither is this
    ```

    ### Real heading

    Real content.
""")


def test_headings_inside_fenced_code_ignored():
    result = filter_readme(FENCED_CODE, "VS Code")
    assert "### This is not a heading" in result
    assert "# Neither is this" in result
    assert "### Real heading" in result


FENCED_4_BACKTICK = dedent("""\
    ## Features

    ````python
    ### Not a heading
    ```
    still inside
    ```
    ````

    ### After code

    After content.
""")


def test_4_backtick_fence_not_closed_by_3():
    """A 4-backtick fence should not be closed by a 3-backtick line."""
    result = filter_readme(FENCED_4_BACKTICK, "VS Code")
    assert "### Not a heading" in result
    assert "still inside" in result
    assert "### After code" in result
    assert "After content." in result


# ---------------------------------------------------------------------------
# Blank line collapsing
# ---------------------------------------------------------------------------


def test_collapses_excessive_blank_lines():
    text = "Line 1\n\n\n\n\nLine 2\n"
    result = filter_readme(text, "VS Code")
    assert "Line 1\n\nLine 2\n" == result


# ---------------------------------------------------------------------------
# Integration: filter the actual README.md
# ---------------------------------------------------------------------------

README_PATH = ROOT / "README.md"


@pytest.mark.skipif(not README_PATH.exists(), reason="README.md not found")
class TestActualReadme:
    def test_vscode_no_other_editor_headings(self):
        text = README_PATH.read_text(encoding="utf-8")
        result = filter_readme(text, "VS Code")
        for editor in KNOWN_EDITORS - {"VS Code"}:
            assert f"### {editor}\n" not in result

    def test_vscode_keeps_vs_code_section(self):
        text = README_PATH.read_text(encoding="utf-8")
        result = filter_readme(text, "VS Code")
        assert "### VS Code" in result

    def test_vscode_strips_all_editors_table(self):
        text = README_PATH.read_text(encoding="utf-8")
        result = filter_readme(text, "VS Code")
        assert "### All editors" not in result

    def test_vscode_strips_paren_suffixes(self):
        text = README_PATH.read_text(encoding="utf-8")
        result = filter_readme(text, "VS Code")
        assert "(VS Code panel)" not in result
        assert "(VS Code + GitHub Copilot)" not in result
        assert "(VS Code commands)" not in result

    def test_every_editor_produces_valid_output(self):
        text = README_PATH.read_text(encoding="utf-8")
        for editor in KNOWN_EDITORS:
            result = filter_readme(text, editor)
            assert f"### {editor}" in result
            assert "## Features" in result
