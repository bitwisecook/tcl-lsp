#!/usr/bin/env python3
"""C-API ownership-contract lint — T2.1 deliverable.

Every public C Tcl API function the runtime ships (declared in the
authored headers, `c-extension-abi.md` §4.1) must carry an ownership /
error-path annotation row in
``docs/design/runtime/c-api-ownership-contract.md``. This gate rejects a
header-declared C-API function with no row (a new export lacking an
ownership annotation), and a row that names a function no header
declares (a stale row or typo).

It is the C-API sibling of ``scripts/check_refcount_contract.py`` (which
gates the *internal* ``tcl_*``/``obj_*`` runtime exports).

What counts as a "shipped C-API function":

- An ``extern <ret> Name(`` function declaration in one of the authored
  headers. ``#define`` macros (``Tcl_NewIntObj``, ``Tcl_PkgProvide``,
  ``Tcl_GetHashValue``, ``Tcl_SetHashValue``) and ``extern`` **data**
  symbols (``tclStubsPtr``, ``tclOOStubsPtr``) are intentionally NOT
  function declarations and are excluded — they carry no per-call
  refcount/error semantics (see the contract doc, "The gate").

When ``runtime/rust/`` lands its ``#[no_mangle] extern "C"`` C-API
exports, extend ``collect_decls`` to also read those so the gate
cross-checks the real exports too.

Exit codes:

- 0: every declared C-API function has a row (and no stale rows), OR
     gaps exist but ``--strict`` was not passed (warning only).
- 1: gaps exist and ``--strict`` was passed.

Usage:

    uv run python scripts/check_c_api_ownership.py
    uv run python scripts/check_c_api_ownership.py --strict
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
HEADERS = [
    REPO / "runtime" / "rust-spike" / "include" / "tcl.h",
    REPO / "runtime" / "rust-spike" / "include" / "tclOO.h",
    REPO / "runtime" / "rust-spike" / "include" / "tclTomMath.h",
]
CONTRACT = REPO / "docs" / "design" / "runtime" / "c-api-ownership-contract.md"

# Matches an `extern <return-type...> Name(` function declaration. The
# return type may span the `extern` keyword and the name (pointers,
# `const`, `struct`, multiple words), so we anchor on `extern`, then take
# the LAST identifier immediately followed by `(` on the declaration.
# `extern const TclOOStubs *tclOOStubsPtr;` has no `(` so never matches
# (data symbol, correctly excluded).
_EXTERN_DECL_RE = re.compile(
    r"\bextern\b[^;{]*?\b([A-Za-z_][A-Za-z0-9_]*)\s*\(",
    re.DOTALL,
)

# A function-row's first cell names one or more functions in code spans:
#   | `Tcl_NewObj` | ... |
#   | `Tcl_MutexLock` / `Tcl_MutexUnlock` | ... |   (combined row)
# We capture every backticked identifier in the FIRST cell. Row collection
# is scoped to the "## Subsystems" section so the category-definition
# tables above it (whose first cells are `borrowed`, `owned`, … — not
# functions) are never mistaken for function rows.
_SUBSYSTEMS_HEADING = "## Subsystems"
_IDENT_RE = re.compile(r"`([A-Za-z_][A-Za-z0-9_]*)`")


def _strip_block_comments(text: str) -> str:
    """Remove /* ... */ comments so a `Foo(` inside prose never matches."""
    return re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)


def collect_decls() -> dict[str, list[str]]:
    """Return ``{function_name: [header basenames declaring it]}``."""
    out: dict[str, list[str]] = {}
    for header in HEADERS:
        if not header.exists():
            continue
        text = _strip_block_comments(header.read_text(encoding="utf-8"))
        for line_or_decl in _EXTERN_DECL_RE.finditer(text):
            name = line_or_decl.group(1)
            out.setdefault(name, [])
            basename = header.name
            if basename not in out[name]:
                out[name].append(basename)
    return out


def collect_rows() -> set[str]:
    """Parse the contract doc; return every function name with a row.

    Scoped to the ``## Subsystems`` section (the per-function tables);
    captures every backticked identifier in each table row's first cell,
    so combined rows (``Tcl_MutexLock`` / ``Tcl_MutexUnlock``) annotate
    all their functions.
    """
    if not CONTRACT.exists():
        return set()
    out: set[str] = set()
    in_subsystems = False
    for line in CONTRACT.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            in_subsystems = line.strip() == _SUBSYSTEMS_HEADING
            continue
        if not in_subsystems or not line.lstrip().startswith("|"):
            continue
        cells = line.split("|")
        # cells[0] is the empty pre-`|` field; cells[1] is the first cell.
        if len(cells) < 2:
            continue
        first_cell = cells[1]
        # Skip the header/separator rows (``| Function |`` / ``|---|``).
        if "Function" in first_cell or set(first_cell.strip()) <= {"-", ":", " "}:
            continue
        out.update(_IDENT_RE.findall(first_cell))
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--strict",
        action="store_true",
        help="exit 1 on any missing/stale row (default: warning only)",
    )
    args = parser.parse_args()

    decls = collect_decls()
    rows = collect_rows()

    missing = sorted(set(decls) - rows)
    # A row may legitimately reference a `mp_*` etc. that is declared; only
    # flag rows that match NO declared function.
    stale = sorted(rows - set(decls))

    if missing:
        print(
            f"ERROR: {len(missing)} shipped C-API functions missing an "
            f"ownership row in {CONTRACT.relative_to(REPO)}:",
            file=sys.stderr,
        )
        for name in missing:
            where = ", ".join(decls[name])
            print(f"  - {name}  (declared in {where})", file=sys.stderr)

    if stale:
        print(
            f"ERROR: {len(stale)} contract rows reference functions no "
            "shipped header declares — stale rows or typos:",
            file=sys.stderr,
        )
        for name in stale:
            print(f"  - {name}", file=sys.stderr)

    covered = len(rows & set(decls))
    total = len(decls)
    if missing or stale:
        print(
            f"\n{covered} of {total} shipped C-API functions annotated "
            f"({100 * covered // max(1, total)}%).",
            file=sys.stderr,
        )
        return 1 if args.strict else 0

    print(
        f"OK: every shipped C-API function has an ownership row "
        f"({total} functions / {len(rows)} rows).",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
