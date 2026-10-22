#!/usr/bin/env python3
"""Apply MachineApplicable clippy suggestions from a JSONL of rustc messages.

Usage: apply_suggestions.py <jsonl> <repo-root> <path-prefix>... [--codes a,b] [--skip a,b]
Only spans whose file starts with one of the prefixes are edited. Overlapping
edits are skipped. Each suggestion's spans are applied all-or-nothing.
"""
import json
import sys
from collections import defaultdict

args = sys.argv[1:]
codes = skip = None
anyapp = "--any" in args
if anyapp:
    args.remove("--any")
if "--codes" in args:
    i = args.index("--codes"); codes = set(args[i + 1].split(",")); del args[i:i + 2]
if "--skip" in args:
    i = args.index("--skip"); skip = set(args[i + 1].split(",")); del args[i:i + 2]
jsonl, root, *prefixes = args

groups = []  # list of edit groups: [(file, byte_start, byte_end, text)]
seen = set()
for line in open(jsonl):
    m = json.loads(line)
    code = (m.get("code") or {}).get("code")
    if codes and code not in codes:
        continue
    if skip and code in skip:
        continue
    for child in m.get("children", []):
        spans = [s for s in child.get("spans", []) if (s.get("suggestion_applicability") == "MachineApplicable" or (anyapp and s.get("suggestion_applicability") in ("MaybeIncorrect", "Unspecified")))
                 and s.get("suggested_replacement") is not None]
        if not spans:
            continue
        if not all(any(s["file_name"].startswith(p) for p in prefixes) for s in spans):
            continue
        key = tuple((s["file_name"], s["byte_start"], s["byte_end"], s["suggested_replacement"]) for s in spans)
        if key in seen:
            continue
        seen.add(key)
        groups.append((code, list(key)))

by_file = defaultdict(list)
accepted = []
taken = defaultdict(list)
for code, edits in groups:
    ok = True
    for f, a, b, _ in edits:
        for (x, y) in taken[f]:
            if a < y and x < b or (a == b == x == y):
                ok = False
    if not ok:
        continue
    for f, a, b, t in edits:
        taken[f].append((a, b))
        by_file[f].append((a, b, t))
    accepted.append(code)

for f, edits in by_file.items():
    path = f"{root}/{f}"
    data = open(path, "rb").read()
    for a, b, t in sorted(edits, key=lambda e: (e[0], e[1]), reverse=True):
        data = data[:a] + t.encode() + data[b:]
    open(path, "wb").write(data)

from collections import Counter
print(len(accepted), "suggestions applied in", len(by_file), "files")
for c, n in Counter(accepted).most_common():
    print(n, c)
