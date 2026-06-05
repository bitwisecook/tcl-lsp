"""Behavioural guard for the LSP version banner.

Boots the real server over JSON-RPC (``python -m server``, the same entry
point editors launch), sends ``initialize``, and asserts the
``serverInfo.version`` it reports back is the *generated* build version —
not the ``"dev"`` fallback.

This is the test that would have caught the regression it guards against:
``server/server.py`` imported ``FULL_VERSION`` via ``from ._build_info``
(i.e. ``server/_build_info.py``), but the build only ever generates
``shared/_build_info.py``.  The relative import raised ``ImportError`` and
the banner silently fell back to ``"dev"`` in every packaged / Makefile
build.  A static "does the source say ``shared._build_info``?" check would
only catch the one spelling I happened to think of; running the server and
reading the version it actually advertises catches *any* way the banner
can regress to ``"dev"`` — wrong import path, broken generation, a
swallowed exception.

Why no existing test caught it: ``shared/_build_info.py`` is gitignored and
only ``make build-info`` creates it, so in a bare checkout *both* the
correct and the buggy import hit the ``except ImportError: "dev"`` fallback
and look identical.  This test makes the build version present first, so
the two paths diverge.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
BUILD_INFO = ROOT / "shared" / "_build_info.py"

# A version that is unmistakably not the "dev" fallback, used only when no
# build-info file exists yet (bare checkout / CI before `make build-info`).
SENTINEL_VERSION = "0.0.0-buildinfo-e2e-probe"


def _render_build_info(version: str) -> str:
    """Mirror the Makefile's BUILD_INFO recipe (scripts/test-slow-stamp.sh)."""
    return (
        '"""Generated at build time — do not edit."""\n\n'
        f'VERSION: str = "{version}"\n'
        f'GIT_DESCRIBE: str = "{version}"\n'
        f'GIT_HASH: str = "0000000"\n'
        f'FULL_VERSION: str = "{version}"\n'
        f'BUILD_TIMESTAMP: str = "1970-01-01T00:00:00Z"\n'
    )


@pytest.fixture(scope="module")
def expected_full_version() -> str:
    """Ensure a build-info file exists and return its FULL_VERSION.

    If the build hasn't generated one (bare checkout), drop in a sentinel so
    the server has a concrete, non-"dev" version to report.  Leaving the file
    in place is safe: it is gitignored and any ``make`` target regenerates it
    (deleting it mid-run would race the parallel pytest workers).
    """
    if not BUILD_INFO.exists():
        BUILD_INFO.write_text(_render_build_info(SENTINEL_VERSION), encoding="utf-8")

    ns: dict[str, object] = {}
    exec(compile(BUILD_INFO.read_text(encoding="utf-8"), str(BUILD_INFO), "exec"), ns)
    full = ns.get("FULL_VERSION")
    assert isinstance(full, str) and full, "shared/_build_info.py has no FULL_VERSION"
    # The whole point is to distinguish the real version from the fallback;
    # if they were equal the test couldn't tell a broken import apart.
    assert full != "dev", "generated FULL_VERSION must differ from the 'dev' fallback"
    return full


class _StdioLspClient:
    """Minimal Content-Length-framed JSON-RPC client over a server subprocess."""

    def __init__(self) -> None:
        env = {**os.environ, "PYTHONPATH": str(ROOT)}
        self.proc = subprocess.Popen(
            [sys.executable, "-m", "server"],
            cwd=str(ROOT),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
        )

    def _send(self, payload: dict) -> None:
        body = json.dumps(payload).encode("utf-8")
        assert self.proc.stdin is not None
        self.proc.stdin.write(b"Content-Length: %d\r\n\r\n%b" % (len(body), body))
        self.proc.stdin.flush()

    def _read_message(self) -> dict | None:
        assert self.proc.stdout is not None
        length = 0
        while True:
            line = self.proc.stdout.readline()
            if not line:
                return None  # EOF — server died
            text = line.decode("utf-8").rstrip("\r\n")
            if not text:
                break  # blank line ends headers
            if text.lower().startswith("content-length:"):
                length = int(text.split(":", 1)[1].strip())
        if length <= 0:
            return None
        body = self.proc.stdout.read(length)
        return json.loads(body.decode("utf-8"))

    def request(self, msg_id: int, method: str, params: dict, timeout: float = 60.0) -> dict:
        self._send({"jsonrpc": "2.0", "id": msg_id, "method": method, "params": params})
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            msg = self._read_message()
            if msg is None:
                raise AssertionError(
                    "server closed stdout before responding; stderr:\n"
                    + self._drain_stderr()
                )
            if msg.get("id") == msg_id:  # skip notifications / other ids
                return msg
        raise AssertionError(f"timed out waiting for response to {method!r}")

    def notify(self, method: str, params: dict | None = None) -> None:
        self._send({"jsonrpc": "2.0", "method": method, "params": params or {}})

    def _drain_stderr(self) -> str:
        try:
            assert self.proc.stderr is not None
            return self.proc.stderr.read(8192).decode("utf-8", "replace")
        except Exception:  # pragma: no cover - best effort diagnostics
            return "<unavailable>"

    def shutdown(self) -> None:
        try:
            self.request(999_999, "shutdown", {}, timeout=10.0)
            self.notify("exit")
        except Exception:  # pragma: no cover
            pass
        try:
            self.proc.wait(timeout=10.0)
        except subprocess.TimeoutExpired:  # pragma: no cover
            self.proc.kill()
            self.proc.wait(timeout=5.0)


def test_initialize_reports_generated_version_not_dev_fallback(expected_full_version: str):
    """The live server's serverInfo.version must be the build version."""
    client = _StdioLspClient()
    try:
        resp = client.request(
            1,
            "initialize",
            {
                "processId": None,
                "rootUri": None,
                "capabilities": {},
                "clientInfo": {"name": "test_build_info_import", "version": "1.0"},
            },
        )
        assert "result" in resp, f"initialize returned an error: {resp!r}"
        server_info = resp["result"].get("serverInfo")
        assert server_info is not None, (
            "initialize result has no serverInfo — cannot verify the version banner"
        )
        reported = server_info.get("version")
        # The banner is f\"v{_version}\"; a broken build-info import makes it \"vdev\".
        assert reported == f"v{expected_full_version}", (
            f"server reported version {reported!r}, expected "
            f"{'v' + expected_full_version!r}. A regression to 'vdev' means a "
            f"module imported build-info from a path the build never generates "
            f"(the server now falls back to 'dev')."
        )
        assert reported != "vdev", "version banner fell back to the 'dev' default"
    finally:
        client.shutdown()


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-v"]))
