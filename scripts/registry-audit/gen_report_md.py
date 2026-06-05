#!/usr/bin/env python3
"""Generate the data-table section of rust-rewrite-registries.md.

Reads the per-group summary JSONs in tmp/registry-audit/ and emits
Markdown: a master parity table plus per-group completeness matrices
(only dimensions that differ). Keeps the audit's lists machine-generated
and reproducible.

Usage: python3 scripts/registry-audit/gen_report_md.py > section.md
"""

from __future__ import annotations

import glob
import json

ORDER = "tcl stdlib tcllib irules iapps tk expect sdc-base synopsys cadence xilinx quartus mentor".split()


def load_summaries() -> dict[str, dict]:
    out = {}
    for p in glob.glob("tmp/registry-audit/*.summary.json"):
        d = json.load(open(p))
        out[d["group"]] = d
    return out


def verdict(d: dict) -> str:
    miss = len(d["missing_in_rust"])
    extra = len(d["extra_in_rust"])
    gaps = [k for k, v in d["completeness"].items() if v["delta"] < 0]
    if miss > 5 or extra > 5:
        return "NAMES DIFFER"
    if any(d["completeness"][k]["delta"] <= -5 for k in gaps):
        return "DATA GAPS"
    if gaps:
        return "minor gaps"
    return "OK"


def main() -> int:
    S = load_summaries()

    print("### Command registries — name parity & verdict\n")
    print("| Registry | Python | Rust | Missing in Rust | Extra in Rust | Verdict |")
    print("|---|--:|--:|--:|--:|---|")
    for g in ORDER:
        d = S[g]
        print(
            f"| `{g}` | {d['python_count']} | {d['rust_count']} | "
            f"{len(d['missing_in_rust'])} | {len(d['extra_in_rust'])} | {verdict(d)} |"
        )
    print()

    # completeness matrices (only non-zero deltas)
    print("### Command registries — data completeness (commands carrying each dimension)\n")
    print("Only dimensions where Python and Rust differ are shown. `py→rust`.\n")
    for g in ORDER:
        d = S[g]
        c = d["completeness"]
        diffs = {k: v for k, v in c.items() if v["delta"] != 0}
        if not diffs:
            print(f"- **`{g}`** — all captured dimensions at parity.")
            continue
        cells = ", ".join(
            f"`{k}` {v['python']}→{v['rust']}"
            for k, v in sorted(diffs.items(), key=lambda kv: kv[1]["delta"])
        )
        print(f"- **`{g}`** — {cells}")
    print()

    # field-value mismatches worth calling out (summary/synopsis text etc.)
    print("### Command registries — value mismatches on common commands\n")
    print("| Registry | field | # mismatched | examples |")
    print("|---|---|--:|---|")
    for g in ORDER:
        fm = S[g]["field_mismatch"]
        # surface only "content" fields, not the structural-count fields that
        # are already covered by the completeness matrix.
        for f in ("summary", "synopsis", "body_kind", "return_type", "arity_min", "arity_max"):
            if f in fm and fm[f]["count"]:
                ex = ", ".join(f"`{x}`" for x in fm[f]["examples"][:3])
                print(f"| `{g}` | {f} | {fm[f]['count']} | {ex} |")
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
