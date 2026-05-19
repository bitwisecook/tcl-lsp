"""Tests for dialect detection from source content."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from shared.dialect import detect_dialect_from_source


def test_shebang_tclsh84():
    assert detect_dialect_from_source("#!/usr/bin/tclsh8.4\nset x 1\n") == "tcl8.4"


def test_shebang_tclsh85():
    assert detect_dialect_from_source("#!/usr/bin/tclsh8.5\nset x 1\n") == "tcl8.5"


def test_shebang_env_tclsh86():
    assert detect_dialect_from_source("#!/usr/bin/env tclsh8.6\nset x 1\n") == "tcl8.6"


def test_shebang_tclsh90():
    assert detect_dialect_from_source("#!/usr/bin/env tclsh9.0\nset x 1\n") == "tcl9.0"


def test_shebang_expect():
    assert detect_dialect_from_source("#!/usr/bin/expect\nspawn ssh\n") == "expect"


def test_shebang_env_expect():
    assert detect_dialect_from_source("#!/usr/bin/env expect\nspawn ssh\n") == "expect"


def test_directive_tcl84():
    assert detect_dialect_from_source("# tcl-dialect: tcl8.4\nset x 1\n") == "tcl8.4"


def test_directive_irules():
    assert (
        detect_dialect_from_source("# tcl-dialect: f5-irules\nwhen HTTP_REQUEST {\n") == "f5-irules"
    )


def test_directive_on_later_line():
    source = "#!/usr/bin/tclsh\n# My script\n# tcl-dialect: tcl8.5\nset x 1\n"
    assert detect_dialect_from_source(source) == "tcl8.5"


def test_directive_takes_priority_over_shebang():
    source = "#!/usr/bin/tclsh8.6\n# tcl-dialect: tcl8.4\nset x 1\n"
    assert detect_dialect_from_source(source) == "tcl8.4"


def test_directive_case_insensitive():
    assert detect_dialect_from_source("# TCL-DIALECT: tcl9.0\nset x 1\n") == "tcl9.0"


def test_directive_crlf_line_endings():
    assert detect_dialect_from_source("# tcl-dialect: tcl8.4\r\nset x 1\r\n") == "tcl8.4"


def test_directive_unknown_dialect_ignored():
    assert detect_dialect_from_source("# tcl-dialect: unknown\nset x 1\n") is None


def test_no_hint_returns_none():
    assert detect_dialect_from_source("set x 1\nputs $x\n") is None


def test_shebang_plain_tclsh_no_version():
    assert detect_dialect_from_source("#!/usr/bin/tclsh\nset x 1\n") is None


def test_directive_beyond_scan_window():
    lines = ["# line\n"] * 10 + ["# tcl-dialect: tcl8.4\n", "set x 1\n"]
    assert detect_dialect_from_source("".join(lines)) is None
