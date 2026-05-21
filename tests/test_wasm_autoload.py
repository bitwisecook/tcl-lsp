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
        wasm = _compile_tcl('set rc [catch {definitely_not_a_command 1 2} m]\nputs "rc=$rc"\n')
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "rc=1\n"

    def test_source_nopkg_behaves_like_plain_source(self):
        """``source -nopkg fileName`` is the undocumented 3-word form the
        real ``::tcl::Pkg::source`` (init.tcl) uses.  Tcl 9 treats it as a
        plain source (the ``-nopkg`` flag only suppresses package-files
        tracking, which the WASM runtime does not maintain).  Sourcing the
        upstream ``parray.tcl`` this way must define ``::parray`` exactly
        as a flagless source would."""
        out = self._run("source -nopkg /tcl-lib/parray.tcl\narray set a {x 9}\nparray a\n")
        assert out == "a(x) = 9\n"


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
        module = wasm_link(prog, library_dir=str(_TCL_LIBRARY) if with_library else None)
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
            'puts [tcl_endOfWord "hello world" 0]\nputs [tcl_startOfNextWord "hello world" 0]\n',
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
            'array set a {x 1}\nset rc [catch {parray a} m]\nputs "rc=$rc"\n',
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
        wasm = self._link("array set a {x 1 y 22}\nparray a\n", tmp_path, with_library=False)
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "a(x) = 1\na(y) = 22\n"

    def test_opt_package_bundled_runs_without_tcl_library(self, tmp_path):
        """``package require opt`` resolves through the package's
        ``pkgIndex.tcl`` (``source``-style ``ifneeded``) — the compiler
        bundles ``optparse.tcl`` and its transitive ``::tcl::Opt*``
        helpers, so the option-parsing proc it defines runs entirely
        self-contained with no run-time library."""
        src = (
            "package require opt\n"
            "::tcl::OptProc seethrough {\n"
            '    {-a -int 1 "first option"}\n'
            '    {-b -int 2 "second option"}\n'
            "} {\n"
            '    puts "a=$a b=$b"\n'
            "}\n"
            "seethrough -a 10 -b 20\n"
            "seethrough\n"
        )
        wasm = self._link(src, tmp_path, with_library=True)
        # No TCL_LIBRARY, no preopen — opt is compiled into the bundle.
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "a=10 b=20\na=1 b=2\n"

    def test_package_require_not_bundled_without_library(self, tmp_path, monkeypatch):
        """With no library the package source can't be discovered, so an
        opt command still errors — proving the compile-time bundling, not
        a runtime fallback, is what resolved it above."""
        monkeypatch.delenv("TCL_LIBRARY", raising=False)
        src = (
            "package require opt\n"
            "set rc [catch {::tcl::OptProc f {} {}} m]\n"
            'puts "rc=$rc"\n'
        )
        wasm = self._link(src, tmp_path, with_library=False)
        _, out = _run_wasm(wasm, capture_stdout=True)
        assert out == "rc=1\n"

    def test_non_allowlisted_command_not_bundled_by_default(self, tmp_path, monkeypatch):
        """A library command outside the validated allowlist is not
        bundled by default (it would fall back to the runtime
        auto-loader), so without a run-time library it still errors."""
        monkeypatch.delenv("TCL_LIBRARY", raising=False)
        # ``history`` is autoloadable but not on the validated allowlist.
        wasm = self._link(
            'set rc [catch {history info} m]\nputs "rc=$rc"\n',
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


class TestStdlibPreludePruning:
    """A bundled file may define several procs; only those statically
    reachable from the program are kept — and only when it's provably
    safe (no eval/source/dynamic dispatch that could invoke a proc by a
    runtime-computed name).  Uses a synthetic library so it exercises the
    pruning logic without depending on the real Tcl tree."""

    def _make_lib(self, tmp_path):
        lib = tmp_path / "lib"
        lib.mkdir()
        (lib / "multi.tcl").write_text(
            "proc alpha {} { return [beta] }\n"
            "proc beta {} { return 2 }\n"
            "proc gamma {} { return 3 }\n"
            "proc delta {} { return 4 }\n"
        )
        (lib / "tclIndex").write_text(
            "# Tcl autoload index file, version 2.0\n"
            + "".join(
                f"set auto_index({c}) [list ::tcl::Pkg::source [file join $dir multi.tcl]]\n"
                for c in ("alpha", "beta", "gamma", "delta")
            )
        )
        return lib

    def _bundled(self, src, lib):
        from core.compiler.lowering import lower_to_ir
        from core.compiler.stdlib_prelude import apply_stdlib_prelude

        module = lower_to_ir(src)
        before = set(module.procedures)
        # allowlist=None: bundle every referenced command (the gate under
        # test is the *pruning* safety check, not the seed allowlist).
        apply_stdlib_prelude(module, lib, allowlist=None)
        return sorted(q.lstrip(":") for q in (set(module.procedures) - before))

    def test_unreachable_procs_pruned(self, tmp_path):
        lib = self._make_lib(tmp_path)
        # alpha calls beta; gamma/delta are unreachable.
        assert self._bundled("puts [alpha]\n", lib) == ["alpha", "beta"]

    def test_single_proc_reachability(self, tmp_path):
        lib = self._make_lib(tmp_path)
        assert self._bundled("puts [gamma]\n", lib) == ["gamma"]

    def test_eval_disables_pruning(self, tmp_path):
        lib = self._make_lib(tmp_path)
        # `eval $x` could invoke any proc by name → keep everything.
        out = self._bundled("set x foo\neval $x\nputs [alpha]\n", lib)
        assert out == ["alpha", "beta", "delta", "gamma"]

    def test_dispatcher_command_disables_pruning(self, tmp_path):
        lib = self._make_lib(tmp_path)
        # `after`'s script arg isn't a [...] substitution the scanner
        # walks → unprovable → keep everything.
        out = self._bundled("after 0 {puts hi}\nputs [alpha]\n", lib)
        assert out == ["alpha", "beta", "delta", "gamma"]

    def test_dynamic_command_name_disables_pruning(self, tmp_path):
        lib = self._make_lib(tmp_path)
        out = self._bundled("set c alpha\n$c\nputs [gamma]\n", lib)
        assert out == ["alpha", "beta", "delta", "gamma"]

    def test_dynamic_command_in_substitution_disables_pruning(self, tmp_path):
        lib = self._make_lib(tmp_path)
        # `[$c]` — a command substitution whose command word is computed
        # at run time — must trip the gate (was previously missed).
        out = self._bundled("set c beta\nputs [gamma][$c]\n", lib)
        assert out == ["alpha", "beta", "delta", "gamma"]

    def test_nested_command_substitution_disables_pruning(self, tmp_path):
        lib = self._make_lib(tmp_path)
        out = self._bundled("puts [gamma][[set c beta]]\n", lib)
        assert out == ["alpha", "beta", "delta", "gamma"]

    def test_multi_segment_file_join_path(self, tmp_path):
        """tclIndex entries with subdirectory file-join segments
        (``[file join $dir sub foo.tcl]``) resolve correctly."""
        from core.compiler.lowering import lower_to_ir
        from core.compiler.stdlib_prelude import apply_stdlib_prelude

        lib = tmp_path / "lib"
        (lib / "sub").mkdir(parents=True)
        (lib / "sub" / "foo.tcl").write_text("proc foo {} { return 42 }\n")
        (lib / "tclIndex").write_text(
            "set auto_index(foo) [list ::tcl::Pkg::source [file join $dir sub foo.tcl]]\n"
        )
        module = lower_to_ir("puts [foo]\n")
        apply_stdlib_prelude(module, lib, allowlist=None)
        assert "::foo" in module.procedures


class TestPackageRequireBundling:
    """``package require NAME`` bundles the package's ``source``-style
    ``pkgIndex.tcl`` entry at compile time.  Synthetic library so the
    discovery + bundling logic is exercised without the real Tcl tree."""

    def _make_lib(self, tmp_path):
        lib = tmp_path / "lib"
        (lib / "mypkg").mkdir(parents=True)
        (lib / "mypkg" / "mypkg.tcl").write_text(
            "proc ::mypkg::hello {} { return hi }\n"
            "proc ::mypkg::unused {} { return nope }\n"
        )
        (lib / "mypkg" / "pkgIndex.tcl").write_text(
            "package ifneeded mypkg 1.0 "
            "[list source -encoding utf-8 [file join $dir mypkg.tcl]]\n"
        )
        return lib

    def _bundled(self, src, lib):
        from core.compiler.lowering import lower_to_ir
        from core.compiler.stdlib_prelude import apply_stdlib_prelude

        module = lower_to_ir(src)
        before = set(module.procedures)
        apply_stdlib_prelude(module, lib, allowlist=None)
        return module, sorted(q.lstrip(":") for q in (set(module.procedures) - before))

    def test_required_package_source_is_bundled(self, tmp_path):
        lib = self._make_lib(tmp_path)
        _, bundled = self._bundled("package require mypkg\nputs [::mypkg::hello]\n", lib)
        assert "mypkg::hello" in bundled

    def test_dynamic_package_name_not_bundled(self, tmp_path):
        """``package require $name`` is a run-time computation — the
        compiler can't see the name, so nothing is bundled (the runtime
        package machinery handles it)."""
        lib = self._make_lib(tmp_path)
        _, bundled = self._bundled("set name mypkg\npackage require $name\n", lib)
        assert bundled == []

    def test_no_require_means_no_bundle(self, tmp_path):
        lib = self._make_lib(tmp_path)
        _, bundled = self._bundled("puts hello\n", lib)
        assert bundled == []
