#!/usr/bin/env python3
"""Per proc: max number of *sibling* if-blocks (same nesting level, same
enclosing block) that conditionally assign the same variable -- the exponent
of the phi_can_undef path explosion."""

import sys, re, collections

path = sys.argv[1]
src = open(path, errors="replace").read()
lines = src.split("\n")
depth = 0
proc = None
# stack of dicts: level -> Counter(var -> count of sibling conditional-set regions)
sib = collections.defaultdict(collections.Counter)
best = {}
setre = re.compile(r"^\s*set\s+([A-Za-z_:][\w:]*)\b")
ifre = re.compile(r"^\s*(if|switch|foreach|while|for)\b")
# We approximate: an `if` at depth d opens a region; any `set V` seen at depth>d
# before the region closes marks V as conditionally written in that region.
open_regions = []  # (depth_at_open, set_of_vars)
for ln in lines:
    if ln.startswith("proc "):
        if proc:
            best[proc] = {(d, v): n for d, ctr in sib.items() for v, n in ctr.items()}
        proc = ln.split()[1]
        sib = collections.defaultdict(collections.Counter)
        open_regions = []
        depth = 0
    if ifre.match(ln):
        open_regions.append([depth, set()])
    m = setre.match(ln)
    if m:
        for r in open_regions:
            r[1].add(m.group(1))
    o = ln.count("{")
    c = ln.count("}")
    depth += o - c
    while open_regions and depth <= open_regions[-1][0]:
        d, vs = open_regions.pop()
        for v in vs:
            sib[d][v] += 1
if proc:
    best[proc] = {(d, v): n for d, ctr in sib.items() for v, n in ctr.items()}
rows = []
for p, m in best.items():
    if not m:
        continue
    (d, v), n = max(m.items(), key=lambda kv: kv[1])
    rows.append((n, d, v, p))
rows.sort(reverse=True)
for n, d, v, p in rows[:10]:
    print(f"{n:5d} sibling conditional-write regions  depth={d}  var={v}  proc={p}")
