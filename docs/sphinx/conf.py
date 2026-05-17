"""Sphinx configuration for the ``f5q`` Python API reference.

Builds the API reference site published to Read the Docs.  The
config picks up every public symbol re-exported from
:mod:`f5q` (which in turn forwards from :mod:`core.bigip.query`)
via ``sphinx.ext.autodoc`` and ``sphinx.ext.autosummary`` so
the on-disk reference can't drift from what ``import f5q``
actually exposes.

Build locally::

    uv pip install -e ".[docs]"
    uv run --extra docs sphinx-build -b html docs/sphinx docs/sphinx/_build/html
    open docs/sphinx/_build/html/index.html

The repo's ``.readthedocs.yaml`` points at this file for the
hosted build.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Make the in-repo packages importable so autodoc can introspect
# them — ``f5q`` and ``core.bigip.query`` are both needed because
# the public surface lives in the latter and is re-exported from the
# former.
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT))

# ---------------------------------------------------------------------------
# Project metadata
# ---------------------------------------------------------------------------

project = "f5q"
author = "tcl-lsp contributors"
copyright = "2026, tcl-lsp contributors"

# Pull the version straight from pyproject.toml so the docs don't
# drift from the wheel.  Sphinx imports this module at build time;
# tomllib has been in stdlib since Python 3.11 and the repo's
# requires-python pin is 3.10+, so guard the import.
try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover — only hit on Python 3.10
    import tomli as tomllib  # type: ignore[no-redef]

with (REPO_ROOT / "pyproject.toml").open("rb") as fh:
    _pyproject = tomllib.load(fh)
release = _pyproject["project"]["version"]
version = ".".join(release.split(".")[:2])

# ---------------------------------------------------------------------------
# Extensions
# ---------------------------------------------------------------------------

extensions = [
    # API reference generation.
    "sphinx.ext.autodoc",
    "sphinx.ext.autosummary",
    "sphinx.ext.napoleon",  # Google / NumPy docstring styles
    "sphinx_autodoc_typehints",  # render type hints as proper cross-refs
    # Cross-linking.
    "sphinx.ext.intersphinx",
    "sphinx.ext.viewcode",  # add ``[source]`` links to every entry
    # Markdown support for the narrative pages.
    "myst_parser",
]

autosummary_generate = True

autodoc_default_options = {
    "members": True,
    "undoc-members": False,
    "show-inheritance": True,
    "member-order": "bysource",
}

# Show ``func(arg: int) -> str`` rather than ``func(arg)`` in
# signatures — the docstring tells the reader *what* the function
# does, the signature tells them *how to call it*.
autodoc_typehints = "signature"
autodoc_typehints_format = "short"

napoleon_google_docstring = True
napoleon_numpy_docstring = True
napoleon_use_admonition_for_examples = False

intersphinx_mapping = {
    "python": ("https://docs.python.org/3", None),
}

# Allow Markdown source files alongside RST so the narrative pages
# stay in the same format the rest of the project's docs use.
source_suffix = {
    ".rst": "restructuredtext",
    ".md": "markdown",
}

myst_enable_extensions = [
    "colon_fence",  # ::: blocks for admonitions
    "deflist",
    "fieldlist",
    "smartquotes",
]

# ---------------------------------------------------------------------------
# Theme
# ---------------------------------------------------------------------------

html_theme = "furo"
html_title = f"f5q {release}"
html_static_path: list[str] = []

html_theme_options = {
    "source_repository": "https://github.com/bitwisecook/tcl-lsp/",
    "source_branch": "main",
    "source_directory": "docs/sphinx/",
    "footer_icons": [],
}

# Cleaner module-index display.
modindex_common_prefix = ["f5q.", "core.bigip.query."]

# Default role for `single-backtick` text is :py:obj: so DSL
# fragments don't get treated as cross-references.
default_role = "py:obj"

# Keep ``__all__``-ordered listings stable.
add_module_names = False
