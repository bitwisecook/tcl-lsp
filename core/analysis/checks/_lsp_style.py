"""LSP-style diagnostic-code registrations.

These five style-only codes (W111 / W112 / W115 / W118 / W120)
were historically defined alongside their emit functions in
``lsp/features/diagnostics.py``.  As part of PYTHON-RETIRE-LSP
the LSP feature directory is being retired, but the codes
themselves are referenced by the diagnostic-codes registry
(`core.common.codes`) and by tests that look up the codes by
name (`tests/test_analyser.py`, `tests/test_incremental_update.py`,
`tests/test_tcllib.py`, `tests/test_diagnostic_phases.py`,
`tests/test_code_actions.py`).

This module exists purely to register the codes via bare
``@diag(...)`` calls so the registry sees them when
``core.common.codes_all`` is imported.  The actual emit
functions stay in ``lsp/features/diagnostics.py`` until the
remainder of the LSP feature tree retires.
"""

from __future__ import annotations

from core.common.codes import diag


def _identity(fn):
    return fn


# W111: Line length.
diag(
    "W111",
    "Line exceeds maximum length (see `tclLsp.style.lineLength`).",
    section="warning",
)(_identity)

# W112: Trailing whitespace.
diag("W112", "Trailing whitespace.", section="warning")(_identity)

# W115: Backslash-newline in comment swallows the next line.
diag(
    "W115",
    "Backslash-newline in comment silently swallows the next line.",
    section="warning",
)(_identity)

# W118: Inconsistent line endings.
diag("W118", "Inconsistent line endings.", section="warning")(_identity)

# W120: Missing `package require`.
diag(
    "W120",
    "Command used without a corresponding `package require`.",
    section="warning",
)(_identity)
