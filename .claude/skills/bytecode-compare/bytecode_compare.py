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

"""Compare our compiler bytecode against tclsh reference disassembly.

*Ours* is the `<name>.golden` beside each fixture in the Rust codegen corpus,
byte-identity gated against live codegen by `codegen_golden.rs`.  *Reference* is
captured from a real tclsh by `scripts/capture/bytecode.sh` into
`tests/bytecode_reference/<version>/` — run that first, or every snippet reports
zero reference instructions.

Usage:
    python3 .claude/skills/bytecode-compare/bytecode_compare.py [-v VERSION] <subcommand> [args...]

Options:
    -v, --version VERSION   Tcl version to compare against (8.4, 8.5, 8.6, 9.0).
                            Default: 9.0

Subcommands:
    all                  Compare all snippets, show detailed diffs
    diff <snippet>       Instruction diff for one snippet (e.g. while-simple)
    summary              One-line per snippet summary
    instructions <snippet>  Side-by-side instruction listing
    refresh              Verify the goldens still match live codegen, then re-compare
    categories           Group differences by category
    versions <snippet>   Compare one snippet against all Tcl versions
"""

from __future__ import annotations

import difflib
import os
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent.parent

# The snippet corpus and *our* disassembly both live in the Rust codegen
# fixtures. Each `<name>.tcl` has a sibling `<name>.golden` holding the
# label-stripped disassembly the emitter produces for it, byte-identity gated by
# `rust/tcl-compiler/tests/codegen_golden.rs`. So the goldens ARE our current
# output whenever that test passes.
SNIPPETS_DIR = (
    REPO_ROOT / "rust" / "tcl-compiler" / "tests" / "fixtures" / "codegen" / "matching"
)

# tclsh reference disassembly, captured by `scripts/capture/bytecode.sh` into
# `<version>/` subdirectories. Generated, not checked in.
REF_BASE = REPO_ROOT / "tests" / "bytecode_reference"

AVAILABLE_VERSIONS = ["8.4", "8.5", "8.6", "9.0"]
DEFAULT_VERSION = "9.0"

# Instruction extraction

# Pattern matching instruction lines: "    (N) opcode ..."
_INSTR_RE = re.compile(r"^\s*\((\d+)\)\s+(.+)$")

# Pattern for ByteCode section headers (both our format and tclsh's)
_BYTECODE_RE = re.compile(r"^ByteCode\s")


def _split_bytecode_sections(disasm_text: str) -> list[str]:
    """Split disassembly text into ByteCode sections.

    Each section starts with a ``ByteCode`` header line.  Returns a list
    of section texts sorted by their instruction content so that proc
    body ordering differences between our output and tclsh's hash-table
    order don't affect comparison.
    """
    sections: list[str] = []
    current_lines: list[str] = []

    for line in disasm_text.splitlines():
        if _BYTECODE_RE.match(line):
            if current_lines:
                sections.append("\n".join(current_lines))
            current_lines = [line]
        else:
            current_lines.append(line)
    if current_lines:
        sections.append("\n".join(current_lines))

    if len(sections) <= 1:
        return sections

    # Keep the first section (top-level) in place and sort the remaining
    # proc body sections by their instruction content so that comparison
    # is independent of proc body ordering.
    top = sections[0]
    procs = sorted(sections[1:], key=_section_sort_key)
    return [top] + procs


def _section_sort_key(section_text: str) -> str:
    """Generate a sort key from the instruction content of a section."""
    instrs: list[str] = []
    for raw in section_text.splitlines():
        m = _INSTR_RE.match(raw.rstrip())
        if m:
            instrs.append(raw.strip())
    return "\n".join(instrs)


def extract_instructions(disasm_text: str) -> list[str]:
    """Extract normalised instruction lines from disassembly text.

    Returns lines like: '(0) push1 0  # "x"'
    Strips block labels, headers, metadata — keeps only instructions.
    Sorts ByteCode sections so proc body ordering does not matter.
    Excludes jumpTable annotation lines (tclsh's multi-line format
    doesn't match our single-line format; actual jumpTable opcodes
    are still compared).
    """
    sections = _split_bytecode_sections(disasm_text)
    combined = "\n".join(sections)

    lines: list[str] = []
    for raw in combined.splitlines():
        raw = raw.rstrip()
        m = _INSTR_RE.match(raw)
        if m:
            lines.append(raw.strip())
        # Skip jumpTable annotation lines — tclsh splits them across
        # multiple lines while we emit a single line; the jumpTable
        # opcode itself is still compared via _INSTR_RE above.
    return lines


def normalise_instruction(line: str) -> str:
    """Normalise an instruction for comparison.

    - Strip comments (# ...)
    - Normalise whitespace
    """
    # Strip trailing comment
    if "\t#" in line:
        line = line[: line.index("\t#")]
    elif "  #" in line:
        idx = line.find("  #")
        if idx > 0:
            line = line[:idx]
    return " ".join(line.split())


def extract_opcodes(disasm_text: str) -> list[str]:
    """Extract just the opcode mnemonics in order."""
    sections = _split_bytecode_sections(disasm_text)
    combined = "\n".join(sections)
    ops: list[str] = []
    for raw in combined.splitlines():
        m = _INSTR_RE.match(raw)
        if m:
            instr_part = m.group(2).strip()
            if "\t#" in instr_part:
                instr_part = instr_part[: instr_part.index("\t#")]
            opcode = instr_part.split()[0]
            ops.append(opcode)
    return ops


# File loading


def load_disasm(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text()


def snippet_names() -> list[str]:
    """Return sorted list of snippet stems (e.g. 'append-scalar')."""
    return sorted(p.stem for p in SNIPPETS_DIR.glob("*.tcl"))


def ours_path(name: str) -> Path:
    """Our disassembly for `name` — the golden beside the fixture."""
    return SNIPPETS_DIR / f"{name}.golden"


def ref_dir(version: str) -> Path:
    return REF_BASE / version


def regenerate_ours(bless: bool = False) -> int:
    """Re-derive our disassembly from live source via the Rust golden test.

    `codegen_golden.rs` compiles every fixture through the Rust pipeline and
    compares the label-stripped disassembly to its `.golden`. Running it plain
    *verifies* the goldens still match live codegen; `BLESS_GOLDEN=1` rewrites
    them after an intentional codegen change.
    """
    env = {"BLESS_GOLDEN": "1"} if bless else {}
    cmd = ["cargo", "test", "-p", "tcl-compiler", "--test", "codegen_golden"]
    proc = subprocess.run(cmd, cwd=REPO_ROOT, env={**os.environ, **env}, check=False)
    return proc.returncode


# Comparison logic


def compare_snippet(name: str, version: str = DEFAULT_VERSION) -> dict:
    """Compare one snippet against a specific Tcl version."""
    ref_path = ref_dir(version) / f"{name}.disasm"

    ours_text = load_disasm(ours_path(name))
    ref_text = load_disasm(ref_path)

    ours_instrs = extract_instructions(ours_text)
    ref_instrs = extract_instructions(ref_text)

    ours_ops = extract_opcodes(ours_text)
    ref_ops = extract_opcodes(ref_text)

    ours_norm = [normalise_instruction(i) for i in ours_instrs]
    ref_norm = [normalise_instruction(i) for i in ref_instrs]

    return {
        "name": name,
        "version": version,
        "ours_instrs": ours_instrs,
        "ref_instrs": ref_instrs,
        "ours_ops": ours_ops,
        "ref_ops": ref_ops,
        "ours_norm": ours_norm,
        "ref_norm": ref_norm,
        "ours_count": len(ours_ops),
        "ref_count": len(ref_ops),
        "match": ours_norm == ref_norm,
        "ops_match": ours_ops == ref_ops,
    }


def categorise_differences(result: dict) -> list[str]:
    """Categorise the differences between ours and tclsh."""
    cats: list[str] = []
    ours_ops = set(result["ours_ops"])
    ref_ops = set(result["ref_ops"])

    if ("storeStk" in ref_ops or "loadStk" in ref_ops) and (
        "storeScalar1" in ours_ops or "loadScalar1" in ours_ops
    ):
        cats.append("variable-access: storeStk/loadStk vs storeScalar1/loadScalar1")

    if "incrStkImm" in ref_ops and "incrScalar1Imm" in ours_ops:
        cats.append("incr-access: incrStkImm vs incrScalar1Imm")

    if result["ours_count"] != result["ref_count"]:
        diff = result["ours_count"] - result["ref_count"]
        sign = "+" if diff > 0 else ""
        cats.append(f"instruction-count: {sign}{diff}")

    ref_only = ref_ops - ours_ops
    ours_only = ours_ops - ref_ops
    if ref_only:
        cats.append(f"ref-only-ops: {', '.join(sorted(ref_only))}")
    if ours_only:
        cats.append(f"ours-only-ops: {', '.join(sorted(ours_only))}")

    ours_done_count = result["ours_ops"].count("done")
    ref_done_count = result["ref_ops"].count("done")
    if ours_done_count > ref_done_count:
        cats.append(
            f"extra-done: {ours_done_count - ref_done_count} extra done instructions"
        )

    return cats


# Output formatting

BOLD = "\033[1m"
RED = "\033[31m"
GREEN = "\033[32m"
YELLOW = "\033[33m"
CYAN = "\033[36m"
DIM = "\033[2m"
RESET = "\033[0m"


def fmt_match(match: bool) -> str:
    if match:
        return f"{GREEN}MATCH{RESET}"
    return f"{RED}MISMATCH{RESET}"


# Subcommands


def cmd_summary(version: str) -> None:
    """One-line per snippet summary."""
    names = snippet_names()
    matches = 0
    total = 0
    for name in names:
        result = compare_snippet(name, version)
        total += 1
        if result["match"]:
            matches += 1
        status = fmt_match(result["match"])
        count_info = f"ours: {result['ours_count']:2d} instrs  {version}: {result['ref_count']:2d} instrs"
        diff = result["ours_count"] - result["ref_count"]
        diff_str = f"({'+' if diff > 0 else ''}{diff})" if diff != 0 else ""
        print(f"  {name:<25s} {status}  {count_info}  {diff_str}")

    print(f"\n{matches}/{total} snippets match tclsh {version}")


def cmd_all(version: str) -> None:
    """Compare all snippets with details on mismatches."""
    names = snippet_names()
    matches = 0
    mismatches: list[dict] = []

    for name in names:
        result = compare_snippet(name, version)
        if result["match"]:
            matches += 1
        else:
            mismatches.append(result)

    print(f"{BOLD}=== Bytecode Comparison: ours vs tclsh {version} ==={RESET}\n")
    cmd_summary(version)

    if mismatches:
        print(f"\n{BOLD}=== Detailed Mismatches ==={RESET}\n")
        for result in mismatches:
            _print_diff(result)
            print()


def cmd_diff(name: str, version: str) -> None:
    """Detailed diff for one snippet."""
    result = compare_snippet(name, version)
    print(f"{BOLD}=== {name} (vs tclsh {version}) ==={RESET}")
    print(f"Status: {fmt_match(result['match'])}")
    print(
        f"Instruction counts: ours={result['ours_count']}, {version}={result['ref_count']}"
    )
    print()
    _print_diff(result)

    cats = categorise_differences(result)
    if cats:
        print(f"\n{BOLD}Categories:{RESET}")
        for cat in cats:
            print(f"  - {cat}")


def _print_diff(result: dict) -> None:
    """Print instruction-level diff."""
    name = result["name"]
    version = result["version"]

    print(f"{BOLD}{name}{RESET}:")

    ours_norm = result["ours_norm"]
    ref_norm = result["ref_norm"]

    diff = list(
        difflib.unified_diff(
            ref_norm,
            ours_norm,
            fromfile=f"{version}/{name}",
            tofile=f"ours/{name}",
            lineterm="",
        )
    )
    if diff:
        for line in diff:
            if line.startswith(("---", "+++")):
                print(f"  {DIM}{line}{RESET}")
            elif line.startswith("@@"):
                print(f"  {CYAN}{line}{RESET}")
            elif line.startswith("-"):
                print(f"  {RED}{line}{RESET}")
            elif line.startswith("+"):
                print(f"  {GREEN}{line}{RESET}")
            else:
                print(f"  {line}")
    else:
        print(f"  {GREEN}(identical after normalisation){RESET}")


def cmd_instructions(name: str, version: str) -> None:
    """Side-by-side instruction listing."""
    result = compare_snippet(name, version)
    ours = result["ours_instrs"]
    ref = result["ref_instrs"]

    print(f"{BOLD}=== {name}: side-by-side (vs tclsh {version}) ==={RESET}")
    print(f"{'tclsh ' + version:<55s} | {'ours':<55s}")
    print("-" * 55 + "-+-" + "-" * 55)

    max_lines = max(len(ours), len(ref))
    for i in range(max_lines):
        left = ref[i] if i < len(ref) else ""
        right = ours[i] if i < len(ours) else ""

        left_norm = normalise_instruction(left) if left else ""
        right_norm = normalise_instruction(right) if right else ""

        if left_norm == right_norm and left_norm:
            marker = " "
        elif not left:
            marker = ">"
        elif not right:
            marker = "<"
        else:
            marker = "!"

        if marker == "!":
            print(f"{RED}  {left:<53s} {marker} {right:<53s}{RESET}")
        elif marker in ("<", ">"):
            print(f"{YELLOW}  {left:<53s} {marker} {right:<53s}{RESET}")
        else:
            print(f"  {left:<53s} {marker} {right:<53s}")


def cmd_refresh(version: str) -> None:
    """Verify our goldens still match live codegen, then show the comparison."""
    print("Verifying our disassembly against live codegen (codegen_golden)...")
    rc = regenerate_ours()
    if rc != 0:
        print(
            "\ncodegen_golden failed: the goldens no longer match what the compiler\n"
            "emits. If the codegen change was intentional, re-bless them with:\n"
            "    BLESS_GOLDEN=1 cargo test -p tcl-compiler --test codegen_golden\n",
            file=sys.stderr,
        )
        raise SystemExit(rc)
    print("Done.\n")
    cmd_summary(version)


def cmd_categories(version: str) -> None:
    """Group differences by category across all snippets."""
    names = snippet_names()
    category_map: dict[str, list[str]] = {}

    for name in names:
        result = compare_snippet(name, version)
        if result["match"]:
            continue
        cats = categorise_differences(result)
        for cat in cats:
            key = cat.split(":")[0]
            category_map.setdefault(key, []).append(name)

    print(f"{BOLD}=== Difference Categories (vs tclsh {version}) ==={RESET}\n")
    if not category_map:
        print(f"  {GREEN}All snippets match!{RESET}")
        return

    for category, snippets in sorted(category_map.items()):
        print(f"{BOLD}{category}{RESET} ({len(snippets)} snippets):")
        for s in snippets:
            result = compare_snippet(s, version)
            cats = [c for c in categorise_differences(result) if c.startswith(category)]
            detail = cats[0] if cats else ""
            print(f"    {s}: {detail}")
        print()


def cmd_versions(name: str) -> None:
    """Compare one snippet against all available Tcl versions."""
    print(f"{BOLD}=== {name}: all versions ==={RESET}\n")

    for version in AVAILABLE_VERSIONS:
        rdir = ref_dir(version)
        if not rdir.exists():
            print(f"  {DIM}tclsh {version}: no reference directory{RESET}")
            continue

        result = compare_snippet(name, version)
        status = fmt_match(result["match"])
        count_info = f"ours: {result['ours_count']:2d} instrs  {version}: {result['ref_count']:2d} instrs"
        diff = result["ours_count"] - result["ref_count"]
        diff_str = f"({'+' if diff > 0 else ''}{diff})" if diff != 0 else ""
        print(f"  tclsh {version}:  {status}  {count_info}  {diff_str}")

    print()
    for version in AVAILABLE_VERSIONS:
        rdir = ref_dir(version)
        if not rdir.exists():
            continue
        result = compare_snippet(name, version)
        if not result["match"]:
            print(f"{BOLD}--- diff vs tclsh {version} ---{RESET}")
            _print_diff(result)
            print()


# Argument parsing


def parse_args() -> tuple[str, list[str]]:
    """Parse -v/--version flag and return (version, remaining_args)."""
    args = sys.argv[1:]
    version = DEFAULT_VERSION

    i = 0
    remaining: list[str] = []
    while i < len(args):
        if args[i] in ("-v", "--version") and i + 1 < len(args):
            version = args[i + 1]
            if version not in AVAILABLE_VERSIONS:
                print(f"Unknown version: {version}")
                print(f"Available: {', '.join(AVAILABLE_VERSIONS)}")
                sys.exit(1)
            i += 2
        else:
            remaining.append(args[i])
            i += 1

    return version, remaining


# Main


def main() -> None:
    version, args = parse_args()

    if not args:
        print(__doc__)
        sys.exit(1)

    cmd = args[0]

    match cmd:
        case "all":
            cmd_all(version)
        case "summary":
            cmd_summary(version)
        case "diff":
            if len(args) < 2:
                print("Usage: bytecode_compare.py diff <snippet_name>")
                sys.exit(1)
            cmd_diff(args[1], version)
        case "instructions":
            if len(args) < 2:
                print("Usage: bytecode_compare.py instructions <snippet_name>")
                sys.exit(1)
            cmd_instructions(args[1], version)
        case "refresh":
            cmd_refresh(version)
        case "categories":
            cmd_categories(version)
        case "versions":
            if len(args) < 2:
                print("Usage: bytecode_compare.py versions <snippet_name>")
                sys.exit(1)
            cmd_versions(args[1])
        case _:
            print(f"Unknown subcommand: {cmd}")
            print(__doc__)
            sys.exit(1)


if __name__ == "__main__":
    main()
