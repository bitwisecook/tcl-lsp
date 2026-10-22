#!/usr/bin/env python3
"""Check generated scratch outputs against their source/profile expectations."""
import json
from pathlib import Path
import re
import sys

tag = sys.argv[1]
root = Path(__file__).resolve().parent
rows = []
files = []
current = None
for line in (root / f"{tag}-oracles.log").read_text().splitlines():
    if line.startswith("FILE "):
        current = line[5:]
        files.append(current)
    match = re.fullmatch(r"  (\S+) exit=(\d+): ?(.*)", line)
    if match:
        rows.append((current, match[1], int(match[2]), match[3]))


def expected_output(name):
    if name.startswith(("n03cafe_", "r_n03_")):
        return "B|" if "_jim" in name else "Aé|"
    if name.startswith("n08_opt_") or name == "seam_inline.tcl":
        return "LOCAL|"
    if name == "seam_extract.tcl":
        return "5|1|"
    if name == "n12_old.tcl":
        return "0|"
    if name == "n12_new.tcl":
        return "1|"
    if name.startswith("n15a_"):
        return "OLD|NEW|"
    if name.startswith("n15b_"):
        return "NEW|"
    if name.startswith("n17_effect_"):
        return "good|"
    if name.startswith(("r_noputs_", "r_setonly_", "r_n07_")):
        return ""
    if name.startswith(("n05_", "r_n05_", "r_plain_", "r_classfree_")):
        return "A|"
    if name.startswith(("r_n04wild_", "r_n04exact_", "n04wild_", "n04exact_")):
        return "1|"
    if name.startswith(("r_n06class_", "n06class_")):
        return "A|B|"
    if name.startswith(("r_n06method_", "n06method_")):
        return "OLD DEST|"
    if name.startswith(("r_n06proc_", "n06proc_")):
        return "P|"
    if name.startswith("n06_sibling_"):
        return "P|Q|"
    return None


matched = []
unexpected = []
generated_files = set()
for file, engine, status, output in rows:
    if f"/out/{tag}/" not in file:
        continue
    generated_files.add(file)
    name = Path(file).name
    profile = re.search(r"_(tcl(?:8\.[456]|9\.[01])|jim)(?:_|\.tcl$)", name)
    if profile:
        wanted = "jimsh" if profile[1] == "jim" else profile[1].replace("tcl", "tclsh")
        if engine != wanted:
            continue
    expected = expected_output(name)
    matched.append((file, engine))
    if expected is None or status != 0 or output != expected:
        unexpected.append({"file": file, "engine": engine, "exit": status,
                           "output": output, "expected": expected})

assert len(rows) == 6 * len(files), (len(rows), len(files))
summary = {"files": len(files), "engine_executions": len(rows),
           "generated_output_files": len(generated_files),
           "matched_profile_checks": len(matched),
           "unexpected_matched_results": unexpected,
           "note": "Every file ran on all six engines; profile-specific edits are judged on their matching engine. Unknown generated fixtures require manual review. This output comparison does not by itself prove edit coverage or collision safety."}
(root / f"{tag}-oracle-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, ensure_ascii=False))
sys.exit(bool(unexpected))
