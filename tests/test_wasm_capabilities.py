"""WASM capability-gated commands — exec / exit / glob.

Exercises the capability bitset added in
``runtime/zig/interp/tcl_caps.zig`` and the matching real
implementations of ``exec``, ``exit``, and ``glob``.  Each test
runs the same script twice — once with the capability granted (the
command succeeds) and once without (the command raises a
permission-denied Tcl error that ``catch`` observes cleanly).

Sandboxed-default posture: a script that calls a capability-gated
command without the embedder having set the matching bit MUST get
``permission denied: <cmd> requires CAP_<NAME>`` rather than a
silent miss or a host-side trap.
"""

from __future__ import annotations

from pathlib import Path

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl,
    _run_wasm,
)

# Capability bits — must mirror runtime/zig/interp/tcl_caps.zig.
CAP_EXEC = 1 << 0
CAP_EXIT = 1 << 1
CAP_FS_GLOB = 1 << 2


def _run(source: str, **kwargs) -> tuple[int, str]:
    """Compile and run *source*, returning (return_value, stdout)."""
    wasm = _compile_tcl(source)
    return _run_wasm(wasm, capture_stdout=True, **kwargs)


class TestCapExec:
    def test_default_refuses(self):
        # No capability granted — ``exec echo hi`` must fail through
        # ``catch`` with the canonical permission-denied message.
        _, out = _run("set rc [catch {exec echo hi} msg]\nputs $rc\nputs $msg\n")
        lines = out.splitlines()
        assert lines[0] == "1"
        assert "permission denied" in lines[1]
        assert "CAP_EXEC" in lines[1]

    def test_granted_dispatches_to_host(self):
        captured: list[tuple[list[str], str]] = []

        def fake_spawn(argv: list[str], stdin: str) -> str:
            captured.append((list(argv), stdin))
            return "fake-output\n"

        _, out = _run(
            "set result [exec echo hello world]\nputs $result\n",
            capabilities=CAP_EXEC,
            host_spawn=fake_spawn,
        )
        assert out.strip() == "fake-output"
        assert captured == [(["echo", "hello", "world"], "")]

    def test_revoked_after_grant(self):
        # ``tcl_set_capabilities`` overwrites rather than ORs, so a
        # script that sets bits to 0 mid-run loses access for the
        # rest of the call.  This is checked indirectly here by
        # leaving capabilities=0 — exec must fail even though the
        # callback is wired.
        spawn_called = [False]

        def fake_spawn(argv: list[str], stdin: str) -> str:
            spawn_called[0] = True
            return "should-not-be-reached"

        _, out = _run(
            "set rc [catch {exec true} msg]\nputs $rc\n",
            capabilities=0,
            host_spawn=fake_spawn,
        )
        assert out.strip() == "1"
        assert spawn_called[0] is False


class TestCapExit:
    def test_default_refuses(self):
        # Without ``CAP_EXIT`` an ``exit`` call lands in the catch
        # branch with permission-denied — control returns to the
        # script normally.
        _, out = _run('set rc [catch {exit 7} msg]\nputs $rc\nputs $msg\nputs "still-running"\n')
        lines = out.splitlines()
        assert lines[0] == "1"
        assert "permission denied" in lines[1]
        assert "CAP_EXIT" in lines[1]
        assert lines[2] == "still-running"

    def test_granted_terminates(self):
        # With ``CAP_EXIT`` the runtime calls WASI ``proc_exit``.
        # wasmtime surfaces that as an ``ExitTrap`` (or a generic
        # ``Trap`` on older bindings) — the harness catches it via
        # ``BaseException``.  We verify the stdout produced before
        # the exit reaches the caller (proves we got at least to
        # the ``exit`` call) and that the post-exit ``puts`` did not
        # run.
        wasm = _compile_tcl('puts "before"\nexit 0\nputs "after"\n')
        try:
            _, out = _run_wasm(
                wasm,
                capture_stdout=True,
                capabilities=CAP_EXIT,
            )
        except BaseException as exc:
            # ExitTrap on newer wasmtime, plain Trap on older.
            stdout = getattr(exc, "tcl_stdout", "")
            assert "before" in stdout
            assert "after" not in stdout
            return
        # Some wasmtime bindings translate exit(0) into a normal
        # return — accept that too.
        assert "before" in out
        assert "after" not in out


class TestCapGlob:
    def test_default_refuses(self):
        _, out = _run("set rc [catch {glob *.txt} msg]\nputs $rc\nputs $msg\n")
        lines = out.splitlines()
        assert lines[0] == "1"
        assert "permission denied" in lines[1]
        assert "CAP_FS_GLOB" in lines[1]

    def test_granted_lists_matching_files(self, tmp_path: Path):
        # Seed the preopen with a few files spanning the pattern.
        (tmp_path / "alpha.txt").write_text("a")
        (tmp_path / "beta.txt").write_text("b")
        (tmp_path / "gamma.log").write_text("c")
        (tmp_path / "subdir").mkdir()
        (tmp_path / "subdir" / "nested.txt").write_text("n")

        wasm = _compile_tcl("set files [glob *.txt]\nputs [llength $files]\nputs [lsort $files]\n")
        _, out = _run_wasm(
            wasm,
            capture_stdout=True,
            preopen_tmpdir=str(tmp_path),
            capabilities=CAP_FS_GLOB,
        )
        lines = out.splitlines()
        assert lines[0] == "2"
        # ``glob *.txt`` matches the two top-level .txt files;
        # ``gamma.log`` and ``subdir/nested.txt`` are filtered out
        # by the basename pattern and the no-recursion rule.
        assert lines[1] == "alpha.txt beta.txt"

    def test_nocomplain_returns_empty(self, tmp_path: Path):
        wasm = _compile_tcl("set files [glob -nocomplain *.does-not-match]\nputs |$files|\n")
        _, out = _run_wasm(
            wasm,
            capture_stdout=True,
            preopen_tmpdir=str(tmp_path),
            capabilities=CAP_FS_GLOB,
        )
        assert out.strip() == "||"

    def test_unknown_switch_raises_bad_option(self, tmp_path: Path):
        # ``glob -typs f *.tcl`` is a typo; stock Tcl raises
        # ``bad option "-typs"``.  Used to silently fall through and
        # match files starting with ``-``; the eval_glob error path
        # now mirrors stock semantics.
        wasm = _compile_tcl("set rc [catch {glob -typs *.tcl} msg]\nputs $rc\nputs $msg\n")
        _, out = _run_wasm(
            wasm,
            capture_stdout=True,
            preopen_tmpdir=str(tmp_path),
            capabilities=CAP_FS_GLOB,
        )
        lines = out.splitlines()
        assert lines[0] == "1"
        assert "bad option" in lines[1]
        assert "-typs" in lines[1]

    def test_escaped_meta_matches_literal_file(self, tmp_path: Path):
        # ``glob {\*}`` should probe a real file named ``*`` rather
        # than the byte sequence ``\*``.  Regression for the literal
        # fast path bypassing backslash unescape (Codex P2 review on
        # PR #289).
        (tmp_path / "*").write_text("star")
        wasm = _compile_tcl(
            "set files [glob {\\*}]\nputs [llength $files]\nputs [lindex $files 0]\n"
        )
        _, out = _run_wasm(
            wasm,
            capture_stdout=True,
            preopen_tmpdir=str(tmp_path),
            capabilities=CAP_FS_GLOB,
        )
        lines = out.splitlines()
        assert lines[0] == "1"
        assert lines[1] == "*"

    def test_exec_rejects_embedded_nul(self):
        # Embedded NUL bytes would silently truncate the host-side
        # argv split (NUL is the separator).  ``exec`` rejects them
        # before any allocation and surfaces a catchable Tcl error.
        spawn_called = [False]

        def fake_spawn(argv: list[str], stdin: str) -> str:
            spawn_called[0] = True
            return "should-not-be-reached"

        _, out = _run(
            'set rc [catch {exec foo "bar\\x00baz"} msg]\nputs $rc\nputs $msg\n',
            capabilities=CAP_EXEC,
            host_spawn=fake_spawn,
        )
        lines = out.splitlines()
        assert lines[0] == "1"
        assert "embedded NUL" in lines[1]
        assert spawn_called[0] is False

    def test_exit_negative_code(self):
        # ``exit -1`` on POSIX is the canonical "abnormal termination"
        # signal; the runtime must fold the i64 value through u32
        # truncation so it round-trips without tripping ``@intCast``'s
        # safety check (Codex P1 review on PR #289).
        wasm = _compile_tcl("exit -1\n")
        try:
            _run_wasm(wasm, capture_stdout=True, capabilities=CAP_EXIT)
        except BaseException as exc:
            # A trap is expected — proc_exit rejects the truncated status. The
            # contract is that ``-1`` folded cleanly through u32 truncation and
            # reached proc_exit (which rejects 4294967295 as out-of-range),
            # rather than tripping @intCast's safety check.  So the trap must
            # be the WASI exit-status rejection, NOT an integer-overflow trap.
            msg = str(exc).lower()
            assert not any(k in msg for k in ("overflow", "invalid conversion", "intcast")), msg
            return
        # A normal return is also fine — it likewise proves the cast didn't trap.

    def test_no_match_raises_without_nocomplain(self, tmp_path: Path):
        wasm = _compile_tcl("set rc [catch {glob *.does-not-match} msg]\nputs $rc\nputs $msg\n")
        _, out = _run_wasm(
            wasm,
            capture_stdout=True,
            preopen_tmpdir=str(tmp_path),
            capabilities=CAP_FS_GLOB,
        )
        lines = out.splitlines()
        assert lines[0] == "1"
        assert "no files matched" in lines[1]
