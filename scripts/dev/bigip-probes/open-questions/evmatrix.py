#!/usr/bin/env python3
"""Full iRule command x event validity matrix, as the rule loader reports it.

  evmatrix.py gen <irule-schema.json> <outdir> [chunk]
      For every event in the appliance's own schema
      (/usr/local/www/docs/irule/irule-schema.json), rules holding CHUNK schema
      commands each, one bare command per line inside `if {0} {...}`. The
      loader reports event context per command independently of argument
      syntax errors, so a bare command name is enough. Nothing executes.
      Writes <id>.conf per rule and index.tsv: id, event, comma-joined commands.

  evmatrix.py regen <loader-output> <index.tsv> <outdir>
      One rule per command for every chunk whose error text was truncated.

  evmatrix.py parse <irule-schema.json> <out.json> <loader-output>:<index.tsv>...
      Reduce the loader messages to {event: {command: verdict}}:
        "invalid"       load error (the rule is refused)
        "invalid-warn"  only the new (JET) checker objects; the legacy checker
                        passes, so mcpd downgrades it to a "not caught before"
                        warning and the rule loads
        "valid"         no context message for the bare command
        "unknown"       its chunk's error text was truncated and no later
                        single-command run covered it
      Later inputs override earlier ones for the cells they cover.

mcpd cuts the error part of its load message at roughly 1 KB; the cut shows
as an error line that stops mid-message just before "Unexpected Error". A
chunk ending that way is truncated and must be re-run smaller.

Appliance run: one tmsh process reading, per rule,
  run util bash -c "echo === <id> <event>"
  load sys config merge file <dir>/<id>.conf
  delete ltm rule tcllsp_oq_<id>
on standard input. A separate tmsh process per load costs ~2 s each.
"""

import json
import pathlib
import re
import sys
from collections import Counter

# A bare `relate_client` / `relate_server` makes the checker itself fail
# (`incr len -1` on an empty length in checkBracesBlock), which aborts the
# whole rule check. They are left out and recorded as a checker defect.
CHECKER_CRASHERS = {"relate_client", "relate_server"}
LINE_OFFSET = (
    3  # the loader numbers the rule body: 1 `when`, 2 `if {0} {`, commands from 3
)
CONTEXT_MARKERS = (
    "not valid in current event context",
    "badEventContext",
    "not valid in RULE_INIT",
    "badRuleInitContext",
)


def load_schema(path):
    d = json.load(open(path))
    events = [o["eventName"] for o in d if "eventName" in o]
    cmds = [
        o["commandName"]
        for o in d
        if "commandName" in o and o["commandName"] not in CHECKER_CRASHERS
    ]
    return events, cmds


def write_rules(out, rules):
    out = pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    for rid, ev, cmds in rules:
        body = "\n".join(cmds)
        (out / f"{rid}.conf").write_text(
            f"ltm rule tcllsp_oq_{rid} {{\nwhen {ev} {{\nif {{0}} {{\n{body}\n}}\n}}\n}}\n"
        )
    (out / "index.tsv").write_text(
        "".join(f"{r}\t{e}\t{','.join(c)}\n" for r, e, c in rules)
    )
    print(len(rules), "rules")


def gen(schema, outdir, size):
    events, cmds = load_schema(schema)
    rules = []
    for n, ev in enumerate(events):
        for k in range(0, len(cmds), size):
            rules.append((f"ev_{n:03d}_{k // size:03d}", ev, cmds[k : k + size]))
    write_rules(outdir, rules)


def read_index(path):
    out = {}
    for line in open(path):
        rid, ev, cmds = line.rstrip("\n").split("\t")
        out[rid] = (ev, cmds.split(","))
    return out


def blocks(loader_out):
    text = open(loader_out, errors="replace").read()
    for block in re.split(r"^=== ", text, flags=re.M)[1:]:
        head, _, body = block.partition("\n")
        yield head.split()[0], body


def truncated(body):
    lines = [l for l in body.splitlines() if l.strip()]
    for i, l in enumerate(lines):
        if l.startswith("Unexpected Error") and i > 0:
            prev = lines[i - 1].rstrip()
            return not prev.endswith("]")
    return False


def regen(loader_out, index, outdir):
    idx = read_index(index)
    rules = []
    for rid, body in blocks(loader_out):
        if truncated(body):
            ev, cmds = idx[rid]
            rules += [(f"{rid}_{j:02d}", ev, [c]) for j, c in enumerate(cmds)]
    write_rules(outdir, rules)


MSG = re.compile(
    r"tcllsp_oq_(ev_[0-9_]+):(\d+): (error|warning): \[(.*)\]\[([^\]]*)\]$"
)


def parse(schema, dest, runs):
    events, cmds = load_schema(schema)
    matrix = {ev: {c: "unknown" for c in cmds} for ev in events}
    still_truncated = []
    for run in runs:
        loader_out, index = run.split(":", 1)
        idx = read_index(index)
        for rid, body in blocks(loader_out):
            ev, rcmds = idx[rid]
            cut = truncated(body)
            found = {}
            for piece in re.split(r"(?=/Common/tcllsp_oq_ev_)", body):
                first = piece.strip().split("\n", 1)[0]
                m = MSG.search(first)
                if not m or m.group(1) != rid:
                    continue
                i = int(m.group(2)) - LINE_OFFSET
                if 0 <= i < len(rcmds) and any(
                    s in m.group(4) for s in CONTEXT_MARKERS
                ):
                    v = "invalid" if m.group(3) == "error" else "invalid-warn"
                    if found.get(rcmds[i]) != "invalid":
                        found[rcmds[i]] = v
            for c in rcmds:
                if c in found:
                    matrix[ev][c] = found[c]
                elif not cut:
                    matrix[ev][c] = "valid"
                else:
                    matrix[ev][c] = "unknown"
            if cut:
                still_truncated.append(rid)
    json.dump(
        {
            "events": events,
            "commands": cmds,
            "excluded": sorted(CHECKER_CRASHERS),
            "matrix": matrix,
        },
        open(dest, "w"),
    )
    print(Counter(v for r in matrix.values() for v in r.values()))


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "gen":
        gen(sys.argv[2], sys.argv[3], int(sys.argv[4]) if len(sys.argv) > 4 else 4)
    elif mode == "regen":
        regen(sys.argv[2], sys.argv[3], sys.argv[4])
    else:
        parse(sys.argv[2], sys.argv[3], sys.argv[4:])
