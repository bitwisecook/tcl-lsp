#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Hold the authored C header to both of its hosts.

`runtime/rust/include/tcl.h` is the one C header an extension compiles
against, and it has two legs. `TCL_HOST_NATIVE` is what `rust/tcl-cshim`
implements and `TCL_HOST_WASM` is what `runtime/rust` exports; each leg
declares the functions its host implements and nothing else, so a call a host
does not implement is a compile error and never a call that fails at run time.
This gate keeps that true in both directions, and keeps the header's
`Tcl_Obj` layout the one the WASM runtime's `TclObj` has.

Two checks:

* **Declarations and exports agree.** Offline, so it runs anywhere.
  Every function a leg declares is exported by its host, and every C-API
  function a host exports is declared by its leg (or is a macro in the header,
  as `Tcl_IncrRefCount` and `Tcl_DecrRefCount` are). The runtime's exports are
  the `#[no_mangle]` functions of `runtime/rust/src/capi.rs`; the shim's are the
  `export_name` functions of `rust/tcl-cshim/src/ffi.rs`.

* **The header compiles for wasm32.** With wasi-sdk's `clang`
  (`$WASI_SDK_PATH`, else `/opt/wasi-sdk`): `tests/c/layout.c`, whose static
  assertions are the 24-byte `Tcl_Obj` layout, under the WASM leg and under the
  target's default; the shim's test extension `pkga.c` under the WASM leg alone,
  which declares everything it calls, and under both legs named at once (the
  union, which checks the source against the header on an ILP32 target and
  names no host), each again as an 8.x source (`-DTCL_MAJOR_VERSION=8`); and
  `doors.c`, the one that reads, writes and evaluates in its caller's frame,
  under both legs, with, as the negative, `doors.c` under the WASM leg alone,
  which must be refused because it calls functions only the shim implements.
  Without wasi-sdk this half is skipped with a note, unless
  `TCL_REQUIRE_WASM_LINK` is set, as it is in the CI job that installs the
  toolchain, where a missing compiler is a failure.

Usage:
    python3 scripts/check_c_extension_wasm.py [-v]
    python3 scripts/check_c_extension_wasm.py --self-test

Exit status 0 when both checks pass (or the second was skipped); 1 otherwise.
`--self-test` runs this script's own regression tests against synthetic
fixtures and never touches the real tree.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
import warnings
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS.parent
sys.path.insert(0, str(SCRIPTS))
from check_c_api_ownership import classify_exports, find_capi_exports

HEADER = REPO_ROOT / "runtime/rust/include/tcl.h"
RUNTIME_CAPI = REPO_ROOT / "runtime/rust/src/capi.rs"
SHIM_FFI = REPO_ROOT / "rust/tcl-cshim/src/ffi.rs"
C_TESTS = REPO_ROOT / "rust/tcl-cshim/tests/c"

#: The two legs, by the macro that selects each.
LEGS = {"wasm": "TCL_HOST_WASM", "native": "TCL_HOST_NATIVE"}

#: The variable that turns a skipped compile check into a failure; the same one
#: the real WASM link honours.
REQUIRE_VAR = "TCL_REQUIRE_WASM_LINK"


# The header's preprocessor, as far as the header uses it.

_COMMENT_RE = re.compile(r"/\*.*?\*/", re.DOTALL)
_DEFINED_RE = re.compile(r"\bdefined\s*(?:\(\s*(\w+)\s*\)|(\w+))")
_EXTERN_RE = re.compile(r"\bEXTERN\b[^;{()]*?\b(\w+)\s*\(")
#: What an expression is made of once `defined` and the macros are replaced:
#: numbers, comparisons, arithmetic, parentheses and Python's spelling of the
#: logical operators. Anything else is not an expression the header writes.
_TOKEN_RE = re.compile(r"\s+|\d+|and\b|or\b|not\b|==|!=|<=|>=|<|>|\(|\)|\+|-")


def _logical_lines(text: str) -> list[str]:
    """The header's lines with comments removed and continuations joined."""
    text = _COMMENT_RE.sub(" ", text)
    return re.sub(r"\\\n", " ", text).splitlines()


def _evaluate(expression: str, macros: dict[str, str]) -> bool:
    """A `#if` expression over `defined(...)`, integers and the macros'
    integer values, which is all the header writes."""
    source = expression
    expression = _DEFINED_RE.sub(
        lambda m: "1" if (m.group(1) or m.group(2)) in macros else "0", expression
    )

    def value(match: re.Match[str]) -> str:
        text = macros.get(match.group(0), "0")
        return text if re.fullmatch(r"\d+", text.strip()) else "0"

    expression = re.sub(r"\b[A-Za-z_]\w*\b", value, expression)
    expression = expression.replace("&&", " and ").replace("||", " or ")
    expression = re.sub(r"!(?!=)", " not ", expression)
    position = 0
    while position < len(expression):
        token = _TOKEN_RE.match(expression, position)
        if token is None:
            raise ValueError(f"unsupported #if expression: {source!r}")
        position = token.end()
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        try:
            return bool(eval(expression, {"__builtins__": {}}, {}))
        except (SyntaxError, TypeError) as error:
            raise ValueError(f"unsupported #if expression: {source!r}") from error


def read_leg(text: str, defined: set[str]) -> tuple[list[str], set[str]]:
    """The functions the header declares (`EXTERN`) and the macros it defines,
    with the names in `defined` predefined as the compiler's command line
    would, in source order."""
    macros: dict[str, str] = {name: "1" for name in defined}
    active_text: list[str] = []
    # Each frame: (parent active, a branch has been taken, this branch active)
    stack: list[tuple[bool, bool, bool]] = []
    active = True
    for raw in _logical_lines(text):
        line = raw.strip()
        if not line.startswith("#"):
            if active:
                active_text.append(raw)
            continue
        directive, _, rest = line[1:].strip().partition(" ")
        directive = directive.strip()
        rest = rest.strip()
        if directive in ("if", "ifdef", "ifndef"):
            if directive == "if":
                taken = active and _evaluate(rest, macros)
            elif directive == "ifdef":
                taken = active and rest in macros
            else:
                taken = active and rest not in macros
            stack.append((active, taken, taken))
            active = taken
        elif directive == "elif":
            parent, was_taken, _ = stack.pop()
            taken = parent and not was_taken and _evaluate(rest, macros)
            stack.append((parent, was_taken or taken, taken))
            active = taken
        elif directive == "else":
            parent, was_taken, _ = stack.pop()
            taken = parent and not was_taken
            stack.append((parent, True, taken))
            active = taken
        elif directive == "endif":
            parent, _, _ = stack.pop()
            active = parent
        elif directive == "define" and active:
            name_match = re.match(r"(\w+)(\([^)]*\))?\s*(.*)", rest)
            if name_match:
                macros[name_match.group(1)] = name_match.group(3)
        elif directive == "undef" and active:
            macros.pop(rest, None)
    if stack:
        raise ValueError("unbalanced #if in the header")
    functions = _EXTERN_RE.findall("\n".join(active_text))
    return functions, {m for m in macros if m not in defined}


def header_legs(text: str) -> dict[str, tuple[list[str], set[str]]]:
    """For each leg, the functions it declares and the macros it defines."""
    return {leg: read_leg(text, {macro}) for leg, macro in LEGS.items()}


# What each host exports.


def runtime_exports(capi_rs: Path) -> list[str]:
    """The C-API functions `runtime/rust/src/capi.rs` exports."""
    return classify_exports(find_capi_exports(capi_rs))[0]


_EXPORT_NAME_RE = re.compile(r'export_name\s*=\s*"(\w+)"')


def shim_exports(ffi_rs: Path) -> list[str]:
    """The functions `rust/tcl-cshim/src/ffi.rs` exports under their C names."""
    return _EXPORT_NAME_RE.findall(ffi_rs.read_text(encoding="utf-8"))


def compare(
    host: str, leg: str, declared: list[str], macros: set[str], exported: list[str]
) -> list[str]:
    """The disagreements between a leg and the host it stands for."""
    problems = []
    for name in sorted(set(declared) - set(exported)):
        problems.append(
            f"the {leg} leg declares {name}, which the {host} does not export: "
            "an extension calling it would not link"
        )
    for name in sorted(set(exported) - set(declared) - macros):
        problems.append(
            f"the {host} exports {name}, which the {leg} leg does not declare: "
            "an extension cannot call it"
        )
    duplicates = sorted({n for n in declared if declared.count(n) > 1})
    for name in duplicates:
        problems.append(f"the {leg} leg declares {name} more than once")
    return problems


def check_declarations(
    header: Path = HEADER, capi: Path = RUNTIME_CAPI, ffi: Path = SHIM_FFI
) -> list[str]:
    legs = header_legs(header.read_text(encoding="utf-8"))
    wasm_declared, wasm_macros = legs["wasm"]
    native_declared, native_macros = legs["native"]
    return compare(
        "runtime", "WASM", wasm_declared, wasm_macros, runtime_exports(capi)
    ) + compare("shim", "native", native_declared, native_macros, shim_exports(ffi))


# The wasm32 compiles.

CLANG_FLAGS = [
    "-fsyntax-only",
    "-std=c99",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-ferror-limit=0",
    f"-I{HEADER.parent}",
]

#: clang's report of a call to a function nothing declared, in the wordings of
#: the releases wasi-sdk has shipped.
_UNDECLARED_RE = re.compile(
    r"(?:implicit declaration of function|call to undeclared function) '(\w+)'"
)


def wasi_sdk() -> Path | None:
    root = Path(os.environ.get("WASI_SDK_PATH", "/opt/wasi-sdk"))
    return root if (root / "bin/clang").is_file() else None


def required() -> bool:
    return os.environ.get(REQUIRE_VAR, "") not in ("", "0")


def compile_c(
    sdk: Path,
    source: Path | str,
    defines: tuple[str, ...] = (),
    undefines: tuple[str, ...] = (),
) -> tuple[bool, str]:
    """Compile a file, or C text given as a string, for wasm32, to check it
    only. Answers whether it compiled, and clang's diagnostics."""
    command = [
        str(sdk / "bin/clang"),
        "--target=wasm32-wasip1",
        f"--sysroot={sdk / 'share/wasi-sysroot'}",
        *CLANG_FLAGS,
        *(f"-D{d}" for d in defines),
        *(f"-U{u}" for u in undefines),
    ]
    if isinstance(source, Path):
        command.append(str(source))
        text = None
    else:
        command += ["-x", "c", "-"]
        text = source
    result = subprocess.run(
        command, input=text, capture_output=True, text=True, check=False
    )
    return result.returncode == 0, result.stderr


#: A call only the WASM leg declares, and one only the native leg does: what
#: shows which leg a compile with no host named got.
WASM_ONLY = "Tcl_NewObj"
NATIVE_ONLY = "Tcl_EvalObjEx"
_PROBES = {
    WASM_ONLY: f"#include <tcl.h>\nTcl_Obj *probe(void) {{ return {WASM_ONLY}(); }}\n",
    NATIVE_ONLY: (
        "#include <tcl.h>\nint probe(Tcl_Interp *interp, Tcl_Obj *script) "
        f"{{ return {NATIVE_ONLY}(interp, script, 0); }}\n"
    ),
}
#: Without `__wasm__` the target is not wasm32 as far as the header can tell.
NOT_WASM = ("__wasm__",)
BOTH = ("TCL_HOST_WASM", "TCL_HOST_NATIVE")


def check_compiles(sdk: Path, native_only: set[str]) -> list[str]:
    layout = C_TESTS / "layout.c"
    pkga = C_TESTS / "pkga.c"
    doors = C_TESTS / "doors.c"
    problems = []
    # (source, defines, undefines, what it shows)
    accepted = (
        (
            layout,
            ("TCL_HOST_WASM",),
            (),
            "the WASM leg declares the layout the runtime's TclObj has",
        ),
        (
            layout,
            (),
            (),
            "the layout is the same whichever leg the target defaults to",
        ),
        (
            pkga,
            ("TCL_HOST_WASM",),
            (),
            (
                "the WASM leg declares every function the test extension calls, so "
                "it compiles against the runtime's leg alone"
            ),
        ),
        (
            pkga,
            ("TCL_HOST_WASM", "TCL_MAJOR_VERSION=8"),
            (),
            "the test extension compiles as an 8.x source against the WASM leg",
        ),
        (
            pkga,
            BOTH,
            (),
            "the test extension compiles for wasm32 against both legs at once",
        ),
        (
            pkga,
            (*BOTH, "TCL_MAJOR_VERSION=8"),
            (),
            "the test extension compiles as an 8.x source against both legs",
        ),
        (
            doors,
            BOTH,
            (),
            (
                "the extension that reaches its caller's frame compiles for wasm32 "
                "against both legs at once"
            ),
        ),
        (
            _PROBES[WASM_ONLY],
            (),
            (),
            f"a wasm32 target with no host named is the WASM leg, which declares {WASM_ONLY}",
        ),
        (
            _PROBES[NATIVE_ONLY],
            (),
            NOT_WASM,
            f"a target that is not wasm32 with no host named is the native leg, which declares {NATIVE_ONLY}",
        ),
    )
    # (source, defines, undefines, what it shows, the functions it is refused for)
    refused = (
        (
            doors,
            ("TCL_HOST_WASM",),
            (),
            (
                "the WASM leg declares no variable or evaluation call, and the "
                "extension that reaches its caller's frame calls the shim's"
            ),
            native_only,
        ),
        (
            _PROBES[NATIVE_ONLY],
            (),
            (),
            f"a wasm32 target with no host named is the WASM leg, which has no {NATIVE_ONLY}",
            {NATIVE_ONLY},
        ),
        (
            _PROBES[WASM_ONLY],
            (),
            NOT_WASM,
            f"a target that is not wasm32 with no host named is the native leg, which has no {WASM_ONLY}",
            {WASM_ONLY},
        ),
    )

    def label(source: Path | str, defines: tuple[str, ...], undefines: tuple[str, ...]):
        name = source.name if isinstance(source, Path) else "a one-line probe"
        flags = [f"-D{d}" for d in defines] + [f"-U{u}" for u in undefines]
        return f"{name} {' '.join(flags)}".rstrip()

    for source, defines, undefines, why in accepted:
        ok, stderr = compile_c(sdk, source, defines, undefines)
        if not ok:
            problems.append(
                f"{label(source, defines, undefines)} no longer compiles for wasm32 "
                f"({why}):\n{stderr.rstrip()}"
            )
    for source, defines, undefines, why, names in refused:
        ok, stderr = compile_c(sdk, source, defines, undefines)
        if ok:
            problems.append(
                f"{label(source, defines, undefines)} compiles for wasm32 and must not ({why})"
            )
        elif not names & set(_UNDECLARED_RE.findall(stderr)):
            problems.append(
                f"{label(source, defines, undefines)} is refused for the wrong reason "
                f"({why}): expected a call to an undeclared one of {sorted(names)}, got:\n"
                f"{stderr.rstrip()}"
            )
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("-v", "--verbose", action="store_true")
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run this script's regression tests and exit",
    )
    args = parser.parse_args()
    if args.self_test:
        return run_self_tests()

    legs = header_legs(HEADER.read_text(encoding="utf-8"))
    problems = check_declarations()
    if args.verbose:
        for leg, (declared, macros) in legs.items():
            print(
                f"{leg}: {len(declared)} declared function(s), {len(macros)} macro(s)"
            )
    sdk = wasi_sdk()
    if sdk is None:
        message = (
            "wasi-sdk is not installed (set WASI_SDK_PATH, or install it to "
            "/opt/wasi-sdk)"
        )
        if required():
            problems.append(
                f"{message}, and {REQUIRE_VAR} requires the wasm32 compile checks"
            )
        else:
            print(
                f"note: the wasm32 compile checks are skipped: {message}; "
                f"set {REQUIRE_VAR}=1 to make that a failure."
            )
    else:
        native_only = set(legs["native"][0]) - set(legs["wasm"][0])
        problems += check_compiles(sdk, native_only)

    if problems:
        print(
            f"{len(problems)} problem(s) with runtime/rust/include/tcl.h:",
            file=sys.stderr,
        )
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    wasm, native = legs["wasm"][0], legs["native"][0]
    print(
        f"OK: the header's WASM leg declares {len(wasm)} function(s), each exported by the "
        f"runtime, and its native leg {len(native)}, each exported by the shim"
        + ("; wasm32 compiles passed" if sdk else "")
    )
    return 0


# Self-tests: plain assertions over synthetic fixtures, as check_c_api_ownership.py does.

_FIXTURE = """\
/* EXTERN void InAComment(void); */
#ifndef TCL_MAJOR_VERSION
#   define TCL_MAJOR_VERSION 9
#endif
#if !defined(TCL_HOST_NATIVE) && !defined(TCL_HOST_WASM)
#   define TCL_HOST_DEFAULTED
#endif
EXTERN void Both(int a,
        int b);
#define Tcl_Counted(o) ((o)->n)
#if defined(TCL_HOST_WASM)
EXTERN char *WasmOnly(void);
#endif
#if defined(TCL_HOST_NATIVE)
EXTERN int NativeOnly(void *p);
#   if TCL_MAJOR_VERSION > 8
EXTERN int NativeNine(void);
#   else
EXTERN int NativeEight(void);
#   endif
#elif defined TCL_HOST_WASM
EXTERN int WasmElif(void);
#else
EXTERN int NeitherHost(void);
#endif
static inline void Inline(void) { }
"""


def _self_test_a_leg_declares_the_shared_part_and_its_own(tmp: Path) -> None:
    wasm, wasm_macros = read_leg(_FIXTURE, {"TCL_HOST_WASM"})
    native, native_macros = read_leg(_FIXTURE, {"TCL_HOST_NATIVE"})
    assert wasm == ["Both", "WasmOnly", "WasmElif"], wasm
    assert native == ["Both", "NativeOnly", "NativeNine"], native
    assert "Tcl_Counted" in wasm_macros and "Tcl_Counted" in native_macros
    assert "TCL_HOST_DEFAULTED" not in wasm_macros, "an explicit host skips the default"
    assert "TCL_HOST_WASM" not in wasm_macros, (
        "the predefined host is not the header's macro"
    )


def _self_test_a_major_version_selects_the_eight_dot_x_branch(tmp: Path) -> None:
    text = _FIXTURE.replace(
        "#   define TCL_MAJOR_VERSION 9", "#   define TCL_MAJOR_VERSION 8"
    )
    native, _ = read_leg(text, {"TCL_HOST_NATIVE"})
    assert native == ["Both", "NativeOnly", "NativeEight"], native


def _self_test_neither_host_takes_the_else_branch(tmp: Path) -> None:
    functions, macros = read_leg(_FIXTURE, set())
    assert functions == ["Both", "NeitherHost"], functions
    assert "TCL_HOST_DEFAULTED" in macros


_SECOND_FIXTURE = """\
#define Kept 1
#define Dropped 1
#undef Dropped
#if defined(Dropped)
EXTERN void Gone(void);
#endif
#if defined(TCL_HOST_WASM)
EXTERN void First(void);
#elif defined(TCL_HOST_WASM)
EXTERN void Second(void);
#endif
#if defined(TCL_HOST_WASM) || defined(TCL_HOST_NATIVE)
EXTERN void Either(void);
#endif
#if defined(Kept) && !defined(Dropped) && (2 + 1) == 3
EXTERN void Arithmetic(void);
#endif
#ifdef Kept
EXTERN void Ifdef(void);
#endif
"""


def _self_test_the_directives_the_header_could_use_are_read(tmp: Path) -> None:
    functions, _ = read_leg(_SECOND_FIXTURE, {"TCL_HOST_WASM"})
    assert functions == ["First", "Either", "Arithmetic", "Ifdef"], functions
    functions, _ = read_leg(_SECOND_FIXTURE, {"TCL_HOST_NATIVE"})
    assert functions == ["Either", "Arithmetic", "Ifdef"], functions


def _self_test_an_expression_the_header_does_not_write_is_an_error(tmp: Path) -> None:
    for text in (
        "#if foo(1)\n#endif\n",
        "#if 1 ? 2 : 3\n#endif\n",
        "#if 2 * 2\n#endif\n",
    ):
        try:
            read_leg(text, set())
        except ValueError:
            continue
        raise AssertionError(f"{text!r} was evaluated")


def _self_test_an_unbalanced_conditional_is_an_error(tmp: Path) -> None:
    try:
        read_leg("#if 1\nEXTERN void F(void);\n", set())
    except ValueError:
        return
    raise AssertionError("an #if with no #endif was accepted")


def _self_test_compare_names_both_directions_of_drift(tmp: Path) -> None:
    problems = compare("runtime", "WASM", ["A", "B"], {"C"}, ["A", "C", "D"])
    assert len(problems) == 2, problems
    assert "declares B" in problems[0] and "does not export" in problems[0], problems
    assert "exports D" in problems[1] and "does not declare" in problems[1], problems
    assert compare("runtime", "WASM", ["A"], set(), ["A"]) == []
    twice = compare("runtime", "WASM", ["A", "A"], set(), ["A"])
    assert twice == ["the WASM leg declares A more than once"], twice


def _self_test_shim_exports_are_the_export_names(tmp: Path) -> None:
    ffi = tmp / "ffi.rs"
    ffi.write_text(
        '#[unsafe(export_name = "Tcl_One")]\npub unsafe extern "C" fn one() {}\n'
        '#[unsafe(export_name = "TclFreeObj")]\npub unsafe extern "C" fn two() {}\n',
        encoding="utf-8",
    )
    assert shim_exports(ffi) == ["Tcl_One", "TclFreeObj"], shim_exports(ffi)


def _self_test_the_shipped_header_has_the_two_legs_it_documents(tmp: Path) -> None:
    legs = header_legs(HEADER.read_text(encoding="utf-8"))
    wasm, native = set(legs["wasm"][0]), set(legs["native"][0])
    assert WASM_ONLY in wasm and WASM_ONLY not in native
    assert NATIVE_ONLY in native and NATIVE_ONLY not in wasm
    assert "Tcl_GetVar2Ex" in native and "Tcl_GetVar2Ex" not in wasm
    assert {"Tcl_CreateObjCommand", "TclFreeObj", "Tcl_GetIntFromObj"} <= wasm & native
    assert "Tcl_DecrRefCount" in legs["wasm"][1], "the refcount operations are macros"


_SELF_TESTS = (
    _self_test_a_leg_declares_the_shared_part_and_its_own,
    _self_test_a_major_version_selects_the_eight_dot_x_branch,
    _self_test_neither_host_takes_the_else_branch,
    _self_test_the_directives_the_header_could_use_are_read,
    _self_test_an_expression_the_header_does_not_write_is_an_error,
    _self_test_an_unbalanced_conditional_is_an_error,
    _self_test_compare_names_both_directions_of_drift,
    _self_test_shim_exports_are_the_export_names,
    _self_test_the_shipped_header_has_the_two_legs_it_documents,
)


def run_self_tests() -> int:
    failed = 0
    with tempfile.TemporaryDirectory(prefix="check-c-extension-wasm-selftest-") as td:
        for test in _SELF_TESTS:
            try:
                test(Path(td))
            except AssertionError as exc:
                failed += 1
                print(f"FAIL: {test.__name__}: {exc}", file=sys.stderr)
            else:
                print(f"ok: {test.__name__}")
    total = len(_SELF_TESTS)
    if failed:
        print(f"\nself-test: {failed}/{total} FAILED", file=sys.stderr)
        return 1
    print(f"\nself-test: {total}/{total} passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
