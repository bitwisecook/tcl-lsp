"""WASM auto-loading of the Tcl standard library via ``TCL_LIBRARY``.

The runtime does not *port* the standard library; instead it is pointed
at the real on-disk library (preopened under WASI) through the
``TCL_LIBRARY`` environment variable.  The first time a command misses
both the proc registry and ``::auto_index``, the runtime sources
``$TCL_LIBRARY/tclIndex`` (setting ``dir`` and injecting the thin
``::tcl::Pkg::source`` loader the real index names) and then resolves
the command through the existing ``::auto_index`` machinery — sourcing
the per-command file (e.g. ``parray.tcl``) on demand.
"""

from __future__ import annotations

from pathlib import Path

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

_REPO_ROOT = Path(__file__).resolve().parent.parent
_TCL_LIBRARY = _REPO_ROOT / "tmp" / "tcl9.0.3" / "library"

_needs_library = pytest.mark.skipif(
    not (_TCL_LIBRARY / "tclIndex").is_file() or not (_TCL_LIBRARY / "parray.tcl").is_file(),
    reason="Tcl 9 library tree not present (run scripts to fetch tmp/tcl9.0.3)",
)


@_needs_library
class TestStdlibAutoLoad:
    def _run(self, src: str) -> str:
        wasm = _compile_tcl(src)
        _, out = _run_wasm(
            wasm,
            capture_stdout=True,
            extra_preopens=((str(_TCL_LIBRARY), "/tcl-lib"),),
            env={"TCL_LIBRARY": "/tcl-lib"},
        )
        return out

    def test_parray_autoloads_from_tcl_library(self):
        """``parray`` is not built in, but auto-loads from the real
        ``parray.tcl`` pointed to by ``TCL_LIBRARY`` and prints the
        array contents."""
        out = self._run("array set a {alpha 1 beta 22 gamma 333}\nparray a\n")
        assert out == "a(alpha) = 1\na(beta)  = 22\na(gamma) = 333\n"

    def test_parray_autoloads_via_dynamic_dispatch(self):
        """The same resolution works when the command name is only known
        at runtime (interpreter dispatch path)."""
        out = self._run("array set a {x 9}\nset cmd parray\n$cmd a\n")
        assert out == "a(x) = 9\n"

    def test_unknown_command_without_library_still_errors(self):
        """With no ``TCL_LIBRARY`` the auto-loader is a no-op, so a
        genuinely unknown command still reports an error rather than
        silently succeeding."""
        wasm = _compile_tcl(
            "set rc [catch {definitely_not_a_command 1 2} m]\nputs \"rc=$rc\"\n"
        )
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "rc=1\n"


@_needs_library
class TestStdlibPrelude:
    """Compile-time bundling of referenced stdlib procs (the preferred
    path): the proc compiles to a WASM function and the binary is
    self-contained — it runs with NO ``TCL_LIBRARY`` and no preopen.
    """

    def _link(self, tcl_src: str, tmp_path, *, with_library: bool):
        from core.compiler.codegen.wasm_link import wasm_link

        prog = tmp_path / "prog.tcl"
        prog.write_text(tcl_src)
        module = wasm_link(
            prog, library_dir=str(_TCL_LIBRARY) if with_library else None
        )
        return module.to_bytes()

    def test_parray_bundled_runs_without_tcl_library(self, tmp_path):
        wasm = self._link(
            "array set a {alpha 1 beta 22 gamma 333}\nparray a\n",
            tmp_path,
            with_library=True,
        )
        # No TCL_LIBRARY, no preopen — the proc is compiled into the bundle.
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "a(alpha) = 1\na(beta)  = 22\na(gamma) = 333\n"

    def test_word_commands_bundle_in_substitutions(self, tmp_path):
        """word.tcl commands are referenced inside ``[...]`` (they return
        values) and depend on init-time globals the file itself sets;
        bundling captures both, so they run self-contained and match the
        reference output."""
        wasm = self._link(
            'puts [tcl_endOfWord "hello world" 0]\n'
            'puts [tcl_startOfNextWord "hello world" 0]\n',
            tmp_path,
            with_library=True,
        )
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "5\n6\n"

    def test_no_library_means_no_bundle(self, tmp_path, monkeypatch):
        """With no library (no param, no compile-time ``TCL_LIBRARY``)
        the prelude is a no-op, so an unbundled ``parray`` still errors —
        proving the bundling, not a fallback, is what resolved it."""
        monkeypatch.delenv("TCL_LIBRARY", raising=False)
        wasm = self._link(
            "array set a {x 1}\nset rc [catch {parray a} m]\nputs \"rc=$rc\"\n",
            tmp_path,
            with_library=False,
        )
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "rc=1\n"

    def test_default_on_via_compile_time_tcl_library_env(self, tmp_path, monkeypatch):
        """With no explicit ``library_dir`` but ``TCL_LIBRARY`` set in the
        *compile-time* environment, bundling is on by default — and the
        resulting binary still runs with no run-time library."""
        monkeypatch.setenv("TCL_LIBRARY", str(_TCL_LIBRARY))
        wasm = self._link(
            "array set a {x 1 y 22}\nparray a\n", tmp_path, with_library=False
        )
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "a(x) = 1\na(y) = 22\n"

    def test_non_allowlisted_command_not_bundled_by_default(self, tmp_path, monkeypatch):
        """A library command outside the validated allowlist is not
        bundled by default (it would fall back to the runtime
        auto-loader), so without a run-time library it still errors."""
        monkeypatch.delenv("TCL_LIBRARY", raising=False)
        # ``history`` is autoloadable but not on the validated allowlist.
        wasm = self._link(
            "set rc [catch {history info} m]\nputs \"rc=$rc\"\n",
            tmp_path,
            with_library=True,
        )
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "rc=1\n"

    def test_only_referenced_files_are_bundled(self, tmp_path, monkeypatch):
        """A program that references no bundleable command pulls in
        nothing — bundle size matches the no-library build."""
        monkeypatch.delenv("TCL_LIBRARY", raising=False)
        src = "puts hello\n"
        with_lib = self._link(src, tmp_path, with_library=True)
        without = self._link(src, tmp_path, with_library=False)
        assert with_lib == without
