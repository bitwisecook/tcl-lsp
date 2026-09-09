# tcl-lsp — a language server and toolchain for Tcl
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Tests for the standalone LSP client's CLI surface.

These cover the argument parser and helpers only; driving a real
`tcl-lsp-server` is the job of the native e2e suite.
"""

from __future__ import annotations

import inspect
import subprocess
import sys
from pathlib import Path

import pytest

CLIENT = Path(__file__).resolve().parent / "lsp_client.py"
sys.path.insert(0, str(CLIENT.parent))

from lsp_client import LspClient


def run_cli(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(CLIENT), *args],
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )


def test_help_renders():
    """argparse %-expands every help string, so a literal % in one crashes."""
    result = run_cli("--help")
    assert result.returncode == 0, result.stderr
    assert "--scan-timeout" in result.stdout


SUBCOMMANDS = [
    "semantic-tokens",
    "diagnostics",
    "format",
    "hover",
    "completion",
    "definition",
    "references",
    "code-lens",
    "code-actions",
    "optimize",
    "symbols",
    "diagram",
    "event-info",
    "command-info",
    "context",
    "all",
    "bench",
    "logs",
]


@pytest.mark.parametrize("subcommand", SUBCOMMANDS)
def test_subcommand_help_renders(subcommand: str):
    result = run_cli(subcommand, "--help")
    assert result.returncode == 0, result.stderr


def test_every_subcommand_is_covered():
    """Keeps SUBCOMMANDS honest against the parser it is meant to exercise."""
    usage = run_cli("--help").stdout
    declared = usage[usage.index("{") + 1 : usage.index("}")].split(",")
    assert sorted(declared) == sorted(SUBCOMMANDS)


def test_no_python_backend_option():
    """The client drives the native server only; there is no backend choice."""
    result = run_cli("--server", "python", "semantic-tokens", "x.tcl")
    assert result.returncode != 0
    assert "{python,rust}" not in run_cli("--help").stdout


def test_workspace_root_is_absolute(tmp_path, monkeypatch):
    """A relative rootUri names a URI authority, not a directory."""
    monkeypatch.chdir(tmp_path)
    (tmp_path / "fixture").mkdir()

    client = LspClient("fixture", launch_cmd=["/nonexistent/tcl-lsp-server"])

    assert client.server_dir == str(tmp_path / "fixture")
    assert Path(client.server_dir).as_uri().startswith("file:///")


def test_launch_cmd_is_required():
    """LspClient spawns what it is given, with no implicit fallback server."""
    launch_cmd = inspect.signature(LspClient.__init__).parameters["launch_cmd"]
    assert launch_cmd.default is inspect.Parameter.empty

    client = LspClient("/tmp", launch_cmd=["/nonexistent/tcl-lsp-server"])
    assert client._launch_cmd == ["/nonexistent/tcl-lsp-server"]
