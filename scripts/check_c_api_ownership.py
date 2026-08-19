#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Enforce the C-API ownership/error contract (issue #1404 item 3).

`docs/design/runtime/c-api-ownership-contract.md` documents one row per C Tcl
API function (`Tcl_*` / `mp_*`) naming its `Tcl_Obj` ownership and error-path
category. Until this script, nothing checked that the contract and the actual
exported surface agreed — the doc's own "Known gap: no enforcement" section
said so explicitly. Two directions of drift are possible; this script closes
the one that is cheaply and reliably checkable from this repository alone:

* **export without a row** — `runtime/rust/src/capi.rs` gains a new
  `#[no_mangle] extern "C" fn Tcl_Whatever(...)` and nobody adds the matching
  ownership row. This is a **hard failure**: "an export cannot land without
  an ownership annotation" is exactly the gap issue #1404 names.

The other direction the contract doc describes — a row naming a function the
real `tcl.h` / `tclOO.h` / `tclTomMath.h` headers never declared, or a header
function with no row at all — needs the actual Tcl C headers to check
against, which this checkout does not carry (fetching them is a separate,
network-capable, disk-heavy step: see the `fetch-tcl-source` skill / the
`tmp/tclX.Y.Z` layout `c-api-ownership-contract.md`'s "Sources transcribed"
line names). This script does that comparison too, but only when it finds a
local Tcl source tree to check against (`--tcl-source PATH`, or
`tmp/tcl*/generic` auto-detected); otherwise it prints that the check was
skipped rather than silently pretending to have covered it. Wire
`--tcl-source` in manually once a source tree is fetched, for the fuller
guarantee.

`capi.rs` also exports a handful of internal bootstrap/test helpers
(`tcl_runtime_create_interp`, `tcl_test_reset_counters`, …) that are not part
of the C Tcl API surface an unmodified extension links against — they are
this runtime's own scaffolding, not something `tcl.h` declares. Those are
recognised by their lower-case-leading-word / `tcl_runtime_` / `tcl_test_`
naming (the real API is exclusively `Tcl_*` / `Tcl3d*` / `mp_*`) and
deliberately excluded, the same way the contract doc excludes macros and
stub-table data symbols.

Usage:
    python3 scripts/check_c_api_ownership.py [--tcl-source PATH] [-v]
    python3 scripts/check_c_api_ownership.py --self-test

Exit status 0 if every real C-API export has an ownership row (and, when a
Tcl source tree was found, every header-declared function has a row and
every row names a real header function); 1 otherwise. `--self-test` runs
this script's own regression tests against synthetic fixtures instead
(no real tree touched) — run it after touching the parsing logic.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CAPI_RS = REPO_ROOT / "runtime/rust/src/capi.rs"
CONTRACT_DOC = REPO_ROOT / "docs/design/runtime/c-api-ownership-contract.md"

#: The real C Tcl API surface uses exactly these name prefixes (see the
#: contract doc's scope note: `tcl.h` + `tclOO.h` + `tclTomMath.h`).
#: Everything else `capi.rs` exports is this runtime's own bootstrap/test
#: scaffolding, not part of what an extension compiles against.
API_PREFIXES = ("Tcl_", "mp_")

#: `capi.rs` exports matching these are known-internal and never expected to
#: carry a contract row. Listed explicitly (rather than just "doesn't start
#: with API_PREFIXES") so a genuinely new non-`Tcl_`/`mp_` export still shows
#: up for a human to classify, instead of silently passing.
KNOWN_INTERNAL_EXPORTS = frozenset(
    {
        "tcl_runtime_create_interp",
        "tcl_runtime_delete_interp",
        "tcl_test_reset_counters",
        "tcl_test_alloc_count",
        "tcl_test_double_free_count",
        "tcl_test_finalize",
    }
)


_FN_LINE_RE = re.compile(
    r'^\s*pub\s+(?:unsafe\s+)?extern\s+"C"\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)'
)
_ATTR_LINE_RE = re.compile(r"^\s*#!?\[")


def find_capi_exports(capi_rs: Path) -> list[str]:
    """Every `#[no_mangle] extern "C" fn NAME` in `capi.rs`, in file order.

    Matches both `pub extern "C" fn` and `pub unsafe extern "C" fn` — the
    `unsafe` marker is orthogonal to whether the function is a real ABI
    export.

    `#[no_mangle]` need not be the *immediately* preceding line: an
    intervening single-line attribute (`#[allow(clippy::missing_safety_doc)]`
    — an ordinary, clippy-encouraged pattern this file's own style uses) or a
    blank line is skipped over. A regex requiring strict line-adjacency
    between `#[no_mangle]` and the `fn` — this function's first
    implementation — silently missed exactly this shape: a real export with
    a real ownership gap, invisible to the gate that exists to catch it. Only
    attribute/blank lines are skipped; anything else (a doc comment, a stray
    brace, actual code) stops the search, so `#[no_mangle]` decorating
    something several statements away is never swept in.
    """
    lines = capi_rs.read_text(encoding="utf-8").splitlines()
    exports: list[str] = []
    for i, line in enumerate(lines):
        if line.strip() != "#[no_mangle]":
            continue
        j = i + 1
        while j < len(lines) and (
            not lines[j].strip() or _ATTR_LINE_RE.match(lines[j])
        ):
            j += 1
        if j < len(lines):
            m = _FN_LINE_RE.match(lines[j])
            if m:
                exports.append(m.group(1))
    return exports


def find_doc_rows(contract_doc: Path) -> list[str]:
    """Every function name given a row in the `## Subsystems` tables.

    A row's `Function` column is the first pipe-delimited cell; most rows
    name one function (`` | `Tcl_NewObj` | ... ``), a few name a closely
    related pair in one cell (`` | `Tcl_MutexLock` / `Tcl_MutexUnlock` | ``)
    — every backtick-quoted token in that first cell counts.
    """
    text = contract_doc.read_text(encoding="utf-8")
    names: list[str] = []
    in_subsystems = False
    for line in text.splitlines():
        if line.startswith("## Subsystems"):
            in_subsystems = True
            continue
        if not in_subsystems:
            continue
        if not line.startswith("|"):
            continue
        cells = line.split("|")
        if len(cells) < 2:
            continue
        first_cell = cells[1]
        if first_cell.strip() in ("Function", "---") or set(first_cell.strip()) <= {
            "-"
        }:
            continue
        names.extend(re.findall(r"`([A-Za-z_][A-Za-z0-9_]*)`", first_cell))
    return names


def find_header_declarations(source_root: Path) -> dict[str, list[str]]:
    """Best-effort `Tcl_*` / `mp_*` prototype scan across the given Tcl
    source tree's public headers (`tcl.h`, `tclOO.h`, `tclTomMath.h`).

    Returns `{header_name: [function, ...]}`. A prototype is recognised as
    a line beginning (after `EXTERN`/`TCLAPI`-style qualifiers) with a
    return-type-ish token sequence ending in a bare `Tcl_Name(` or `mp_name(`
    — deliberately permissive, since `tcl.h` uses several export-qualifier
    macros across releases and this is a best-effort cross-check, not the
    enforced half of this script (see the module docstring).
    """
    headers = {
        "tcl.h": None,
        "tclOO.h": None,
        "tclTomMath.h": None,
    }
    for name in list(headers):
        candidates = list(source_root.rglob(name))
        if candidates:
            headers[name] = candidates[0]

    proto_re = re.compile(r"\b((?:Tcl_|mp_)[A-Za-z0-9_]*)\s*\(")
    out: dict[str, list[str]] = {}
    for name, path in headers.items():
        if path is None:
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        found = []
        for line in text.splitlines():
            stripped = line.strip()
            # Skip macro bodies (`#define Tcl_Foo(...)`) — those are the
            # documented-elsewhere macros the contract doc excludes, not
            # real exported symbols.
            if stripped.startswith("#"):
                continue
            m = proto_re.search(stripped)
            if m:
                found.append(m.group(1))
        out[name] = sorted(set(found))
    return out


def find_tcl_source(explicit: Path | None) -> Path | None:
    if explicit is not None:
        return explicit if explicit.is_dir() else None
    tmp = REPO_ROOT / "tmp"
    if not tmp.is_dir():
        return None
    for candidate in sorted(tmp.glob("tcl*")):
        if (candidate / "generic").is_dir():
            return candidate
    return None


# --------------------------------------------------------------------------
# Self-tests. This script has no pytest/unittest harness of its own (it is a
# standalone dev tool, matching the other scripts/ tools' style), so its
# regression coverage lives here as plain assert-based checks against
# synthetic fixtures, run with `--self-test` rather than against the real
# tree. Run these after touching find_capi_exports/find_doc_rows.


def _write(dir_path: Path, name: str, content: str) -> Path:
    p = dir_path / name
    p.write_text(content, encoding="utf-8")
    return p


def _self_test_finds_a_plain_export(tmp_dir: Path) -> None:
    p = _write(
        tmp_dir,
        "plain.rs",
        '#[no_mangle]\npub extern "C" fn Tcl_NewObj() -> *mut TclObj {\n    todo!()\n}\n',
    )
    assert find_capi_exports(p) == ["Tcl_NewObj"], find_capi_exports(p)


def _self_test_finds_an_unsafe_export(tmp_dir: Path) -> None:
    p = _write(
        tmp_dir,
        "unsafe_export.rs",
        '#[no_mangle]\npub unsafe extern "C" fn Tcl_GetString(o: *mut TclObj) -> *mut c_char {\n'
        "    todo!()\n}\n",
    )
    assert find_capi_exports(p) == ["Tcl_GetString"], find_capi_exports(p)


def _self_test_finds_an_export_behind_an_intervening_attribute(tmp_dir: Path) -> None:
    """Regression test for the exact gap an adversarial review found: an
    `#[allow(...)]` between `#[no_mangle]` and the fn item — an ordinary,
    clippy-encouraged pattern this file's own style uses — must not hide
    the export from the ownership-contract gate."""
    p = _write(
        tmp_dir,
        "sneaky.rs",
        "#[no_mangle]\n"
        "#[allow(clippy::missing_safety_doc)]\n"
        'pub unsafe extern "C" fn Tcl_SneakyExportBehindAnotherAttribute() {\n'
        "    todo!()\n}\n",
    )
    assert find_capi_exports(p) == ["Tcl_SneakyExportBehindAnotherAttribute"], (
        find_capi_exports(p)
    )


def _self_test_finds_an_export_behind_several_intervening_attributes(
    tmp_dir: Path,
) -> None:
    p = _write(
        tmp_dir,
        "sneakier.rs",
        "#[no_mangle]\n"
        "#[allow(clippy::missing_safety_doc)]\n"
        "#[allow(non_snake_case)]\n"
        "\n"
        'pub unsafe extern "C" fn Tcl_StillFound() {\n'
        "    todo!()\n}\n",
    )
    assert find_capi_exports(p) == ["Tcl_StillFound"], find_capi_exports(p)


def _self_test_does_not_attach_no_mangle_across_real_code(tmp_dir: Path) -> None:
    """`#[no_mangle]` decorating a `static`, with an unrelated fn later in
    the file, must not have that later fn misattributed to it."""
    p = _write(
        tmp_dir,
        "static_then_fn.rs",
        "#[no_mangle]\n"
        "pub static SOME_TABLE: [u8; 4] = [0, 0, 0, 0];\n"
        "\n"
        'pub extern "C" fn Tcl_NotDecorated() {\n'
        "    todo!()\n}\n",
    )
    assert find_capi_exports(p) == [], find_capi_exports(p)


def _self_test_doc_rows_handle_a_combined_heading_cell(tmp_dir: Path) -> None:
    p = _write(
        tmp_dir,
        "contract.md",
        "## Subsystems\n\n"
        "### Threading\n\n"
        "| Function | Obj args | Return | Errors | Notes |\n"
        "|---|---|---|---|---|\n"
        "| `Tcl_MutexLock` / `Tcl_MutexUnlock` | n/a | `void` | `no-error` | |\n",
    )
    assert find_doc_rows(p) == ["Tcl_MutexLock", "Tcl_MutexUnlock"], find_doc_rows(p)


def _self_test_doc_rows_ignore_tables_before_subsystems(tmp_dir: Path) -> None:
    p = _write(
        tmp_dir,
        "contract2.md",
        "## Categories\n\n"
        "| Category | Meaning |\n"
        "|---|---|\n"
        "| `borrowed` | Caller keeps its reference. |\n\n"
        "## Subsystems\n\n"
        "| Function | Obj args | Return | Errors | Notes |\n"
        "|---|---|---|---|---|\n"
        "| `Tcl_NewObj` | n/a | `fresh_zero` | `no-error` | |\n",
    )
    assert find_doc_rows(p) == ["Tcl_NewObj"], find_doc_rows(p)


_SELF_TESTS = (
    _self_test_finds_a_plain_export,
    _self_test_finds_an_unsafe_export,
    _self_test_finds_an_export_behind_an_intervening_attribute,
    _self_test_finds_an_export_behind_several_intervening_attributes,
    _self_test_does_not_attach_no_mangle_across_real_code,
    _self_test_doc_rows_handle_a_combined_heading_cell,
    _self_test_doc_rows_ignore_tables_before_subsystems,
)


def run_self_tests() -> int:
    failed = 0
    with tempfile.TemporaryDirectory(prefix="check-c-api-ownership-selftest-") as td:
        tmp_dir = Path(td)
        for test in _SELF_TESTS:
            try:
                test(tmp_dir)
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


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument(
        "--tcl-source",
        type=Path,
        default=None,
        help="a fetched Tcl source tree (containing generic/tcl.h etc.) to also "
        "check header declarations against; auto-detected under tmp/ if omitted",
    )
    ap.add_argument(
        "-v", "--verbose", action="store_true", help="list excluded/matched names too"
    )
    ap.add_argument(
        "--self-test",
        action="store_true",
        help="run this script's own regression tests against synthetic fixtures "
        "(does not touch the real tree) and exit",
    )
    args = ap.parse_args()

    if args.self_test:
        return run_self_tests()

    if not CAPI_RS.is_file():
        print(f"error: {CAPI_RS} not found", file=sys.stderr)
        return 2
    if not CONTRACT_DOC.is_file():
        print(f"error: {CONTRACT_DOC} not found", file=sys.stderr)
        return 2

    exports = find_capi_exports(CAPI_RS)
    doc_rows = find_doc_rows(CONTRACT_DOC)
    doc_names = set(doc_rows)

    api_exports = [n for n in exports if n.startswith(API_PREFIXES)]
    unclassified = [
        n
        for n in exports
        if not n.startswith(API_PREFIXES) and n not in KNOWN_INTERNAL_EXPORTS
    ]
    excluded = [n for n in exports if n in KNOWN_INTERNAL_EXPORTS]

    ok = True

    if args.verbose:
        print(
            f"{len(exports)} #[no_mangle] export(s) in {CAPI_RS.relative_to(REPO_ROOT)}:"
        )
        print(f"  {len(api_exports)} real C-API export(s) (Tcl_*/mp_*)")
        print(
            f"  {len(excluded)} known-internal export(s) excluded: {sorted(excluded)}"
        )
        print(
            f"{len(doc_rows)} row-name occurrence(s) in {CONTRACT_DOC.relative_to(REPO_ROOT)}"
        )
        print()

    if unclassified:
        ok = False
        print(
            f"{len(unclassified)} capi.rs export(s) match neither the real C-API naming "
            f"({'/'.join(p + '*' for p in API_PREFIXES)}) nor KNOWN_INTERNAL_EXPORTS in this "
            "script — classify them (add to KNOWN_INTERNAL_EXPORTS if genuinely internal, "
            "otherwise give them a contract row):"
        )
        for n in unclassified:
            print(f"  ? {n}")

    undocumented = [n for n in api_exports if n not in doc_names]
    if undocumented:
        ok = False
        print(
            f"{len(undocumented)} C-API export(s) in capi.rs have no ownership row in "
            f"{CONTRACT_DOC.relative_to(REPO_ROOT)}:"
        )
        for n in undocumented:
            print(f"  + {n}")

    source_root = find_tcl_source(args.tcl_source)
    if source_root is None:
        print(
            "note: no local Tcl source tree found (pass --tcl-source, or fetch one under "
            "tmp/ via the fetch-tcl-source skill) — skipping the header-declaration cross-check; "
            "the capi.rs <-> doc check above still ran and is the enforced half."
        )
    else:
        declared = find_header_declarations(source_root)
        all_declared: set[str] = set()
        for header, names in declared.items():
            if not names:
                print(f"note: {header} not found under {source_root}, skipping")
                continue
            all_declared.update(names)
        if all_declared:
            missing_rows = sorted(n for n in all_declared if n not in doc_names)
            stale_rows = sorted(
                n
                for n in doc_names
                if n not in all_declared and n.startswith(API_PREFIXES)
            )
            if missing_rows:
                ok = False
                print(
                    f"{len(missing_rows)} header-declared function(s) under {source_root.name} "
                    f"have no row in {CONTRACT_DOC.relative_to(REPO_ROOT)}:"
                )
                for n in missing_rows:
                    print(f"  + {n}")
            if stale_rows:
                ok = False
                print(
                    f"{len(stale_rows)} row(s) in {CONTRACT_DOC.relative_to(REPO_ROOT)} name a "
                    f"function {source_root.name}'s headers never declared (typo, or renamed/"
                    "retired API):"
                )
                for n in stale_rows:
                    print(f"  - {n}")

    if ok:
        print(
            f"OK: {len(api_exports)} C-API export(s) in capi.rs each have an ownership row"
            + (
                f"; {source_root.name} header cross-check also passed"
                if source_root
                else ""
            )
            + "."
        )
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
