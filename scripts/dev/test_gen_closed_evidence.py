# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Tests for the pure parts of `gen_closed_evidence.py`."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import gen_closed_evidence as gen  # noqa: E402

LADDER = ["8.4", "8.5", "8.6", "9.0", "9.1"]


def test_a_contiguous_run_is_one_window_open_at_the_newest_release():
    assert gen.windows(["8.5", "8.6", "9.0", "9.1"], LADDER) == [("8.5", None)]


def test_a_gap_splits_the_releases_into_windows():
    assert gen.windows(["8.4", "8.6", "9.0", "9.1"], LADDER) == [("8.4", "8.5"), ("8.6", None)]
    assert gen.windows(["9.0"], LADDER) == [("9.0", "9.1")]
    assert gen.windows([], LADDER) == []


def test_a_window_is_spelled_as_rust():
    assert gen.rust_window(("8.4", "8.5")) == '("8.4", Some("8.5"))'
    assert gen.rust_window(("8.6", None)) == '("8.6", None)'


def test_a_qualified_dict_row_reads_its_ensemble_line():
    assert gen.evidence_key("::tcl::dict::get", "") == "dict get"
    assert gen.evidence_key("string", "length") == "string length"
    assert gen.evidence_key("::tcl::mathop::+", "") == "tcl::mathop::+"


def test_rows_their_authors_leave_open_are_never_closed():
    assert ("tcl", "dict", "create") in gen.OPEN_BY_AUTHOR
