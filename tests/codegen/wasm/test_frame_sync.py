"""Profile assertions for the frame-sync path.

These tests compile small Tcl programs and inspect the emitted WAT to
verify the var-escape analysis actually elides frame-sync work for
procs that cannot leak a variable by name. Regressions here mean the
escape analysis is being too conservative or the sync path has
stopped consulting the summary.
"""

from __future__ import annotations

import textwrap

from core.compiler.codegen.wasm import wasm_codegen_module
from core.compiler.compilation_unit import compile_source


def _compile_wat(source: str) -> str:
    cu = compile_source(textwrap.dedent(source))
    module = wasm_codegen_module(cu.cfg_module, cu.ir_module)
    return module.to_wat()


def _proc_body(wat: str, qname: str) -> str:
    """Return the WAT text of the function whose name is ``qname``."""
    marker = f"(func ${qname}"
    start = wat.index(marker)
    # A function definition ends at the matching top-level ``)``.
    # Each outer ``(func …)`` sits at 2-space indent, so find the
    # next line that starts with two spaces and ``)``.
    end = wat.index("\n  )\n", start) + 5
    return wat[start:end]


def _import_call_site(wat: str, import_name: str) -> str:
    """Return the ``call <idx>`` substring for the named import.

    WAT lists imports in order at the top of the module; each import
    occupies the next available function index.  To check whether a
    proc body calls the import, we resolve its index here and search
    the proc body for ``call <idx>``.
    """
    lines = wat.splitlines()
    idx = 0
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("(import "):
            if f"$imp_{import_name} " in stripped:
                return f"call {idx}"
            idx += 1
    raise AssertionError(f"{import_name!r} not found in module imports")


def _proc_body_without_prologue(body: str) -> str:
    """Strip the parameter-sync prologue from a proc body.

    The prologue always contains ``call <frame_push_idx>`` followed by
    one ``call <local_set_idx>`` per parameter.  We count "real" body
    calls by trimming everything up to and including the last prologue
    local_set — the caller can then assert on the remainder.
    """
    # Trim up to (and including) the ``drop`` that follows frame_push.
    # After that, param mirrors are a sequence of obj_new_string / local.get /
    # call <local_set> / drop quartets.  The cleanest heuristic: the
    # body emits its first real statement after the last prologue
    # ``drop`` that closes a local_set call.  We return from the
    # first line that isn't part of that pattern.
    lines = body.splitlines()
    # Find the frame_push site: first ``call`` followed by ``drop``.
    i = 0
    while i < len(lines) - 1:
        if lines[i].strip().startswith("call ") and lines[i + 1].strip() == "drop":
            i += 2
            break
        i += 1
    # Skip prologue local_set quartets (i32.const idx, i32.const 1,
    # call <obj_new_string>, local.get <p>, call <local_set>, drop).
    # Heuristic: keep advancing while we see a line containing
    # ``local.get <param_slot>`` followed by ``call`` followed by
    # ``drop``.
    while i + 2 < len(lines):
        block = [lines[i].strip(), lines[i + 1].strip(), lines[i + 2].strip()]
        if (
            block[0].startswith("local.get ")
            and block[1].startswith("call ")
            and block[2] == "drop"
        ):
            i += 3
            continue
        if block[0].startswith("i32.const ") and block[1].startswith("i32.const "):
            # Still inside the obj_new_string prefix for another
            # prologue entry; skip it.
            i += 1
            continue
        break
    return "\n".join(lines[i:])


class TestPristineProc:
    def test_arithmetic_proc_emits_no_local_set(self):
        wat = _compile_wat(
            """
            proc add {a b} {
                set sum [expr {$a + $b}]
                return $sum
            }
            """
        )
        body_full = _proc_body(wat, "::add")
        body = _proc_body_without_prologue(body_full)
        local_set_call = _import_call_site(wat, "local_set")
        assert local_set_call not in body, (
            "pristine arithmetic proc should not mirror any var to the frame:\n" + body_full
        )

    def test_list_proc_emits_no_local_set(self):
        wat = _compile_wat(
            """
            proc reverse_pair {xs} {
                set a [lindex $xs 0]
                set b [lindex $xs 1]
                return [list $b $a]
            }
            """
        )
        body_full = _proc_body(wat, "::reverse_pair")
        body = _proc_body_without_prologue(body_full)
        local_set_call = _import_call_site(wat, "local_set")
        assert local_set_call not in body, (
            "list-manipulation proc with no escape should not touch the frame:\n" + body_full
        )


class TestEscapedProc:
    def test_upvar_proc_does_route_through_frame(self):
        # Negative control: a proc that DOES use upvar must still
        # see its escaped var routed through the runtime.  Today the
        # upvar #0 path uses ``tcl_global_set``; a future upvar 1
        # implementation may use ``tcl_local_set`` via a named-alias
        # installer.  Either way, some runtime call beyond trivial
        # arithmetic must appear.
        wat = _compile_wat(
            """
            proc setter {val} {
                upvar #0 target local
                set local $val
            }
            """
        )
        body_full = _proc_body(wat, "::setter")
        gset_call = _import_call_site(wat, "global_set")
        assert gset_call in body_full, (
            "upvar-escaped proc must route the aliased write through the global table:\n"
            + body_full
        )

    def test_eval_literal_proc_routes_touched_names(self):
        # ``eval {set x 99}`` forces x to FRAME via the intraprocedural
        # pass.  The resulting codegen must write x through the frame
        # so the fallback's interpreted set can see it.
        wat = _compile_wat(
            """
            proc do_eval {} {
                set x 1
                set y 2
                eval {set x 99}
                return $x
            }
            """
        )
        body_full = _proc_body(wat, "::do_eval")
        body = _proc_body_without_prologue(body_full)
        local_set_call = _import_call_site(wat, "local_set")
        assert local_set_call in body, (
            "eval-escaped proc must mirror x into the frame:\n" + body_full
        )
