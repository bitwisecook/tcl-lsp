"""External test directory conftest.

Options (``--tcl9-report``, ``--tcl9-required``) and the
``record_tcl9_result`` helper live in the root
``tests/conftest.py`` so pytest discovers the addoption hook at
rootdir. This file re-exports the helper for local convenience.
"""

from __future__ import annotations

from tests.conftest import record_tcl9_result  # re-export

__all__ = ["record_tcl9_result"]
