#!/usr/bin/env python3
"""Join the BIG-IP iRule surface into one model.

Inputs (all derived from one BIG-IP build; keep them and the output outside
this repository, they are derived from F5 source and binaries):

  --grammar   grammar.json from render-irule-grammar.tcl
  --oracle    directory written by irule-oracle.tcl: oracle.tsv (event,
              command, verdict, profile requirement, licence, proxy) plus
              events.tsv, hierarchy.tsv, cat_events.tsv, nsmap.tsv, disabled.tsv
  --schema    /usr/local/www/docs/irule/irule-schema.json from the appliance
  --tmm       TMM command registration table (name, handler, clientdata)
  --suspend   optional: TSV name<TAB>handler<TAB>depth<TAB>via<TAB>confidence
              for TMM handlers that can return Tcl code 5 (F5's suspend
              code). Confidence: "handler" (a dedicated handler returns 5),
              "callee-depth-N", or "shared-dispatcher" (one handler serves
              several commands, so the tag may belong to a sibling).
  --measured  optional: the measured loader matrix (results/21.1-event-matrix.json)
              to report the oracle's agreement
  --out       model.json

The model's verdicts are the load-time checker's: "valid", "invalid" (load
error) or "invalid-warn" (loads with a JET-only warning), for the bare
command. Profile requirements are the dependencies the checker saves for
mcpd to check when the rule is attached to a virtual server.
"""
import argparse
import collections
import json
import pathlib


def tsv(path):
    for line in open(path):
        yield line.rstrip("\n").split("\t")


def main():
    ap = argparse.ArgumentParser()
    for a in ("grammar", "oracle", "schema", "tmm", "out"):
        ap.add_argument("--" + a, required=True)
    ap.add_argument("--suspend")
    ap.add_argument("--measured")
    a = ap.parse_args()
    od = pathlib.Path(a.oracle)

    grammar = json.load(open(a.grammar))
    schema = json.load(open(a.schema))
    doc_cmds = {o["commandName"]: o for o in schema if "commandName" in o}
    doc_events = {o["eventName"]: o for o in schema if "eventName" in o}
    tmm = {}
    for name, fn, cd, _ in tsv(a.tmm):
        tmm.setdefault(name, []).append(fn)

    events = {}
    for ev, cats, profs in tsv(od / "events.tsv"):
        events[ev] = {"categories": cats.split(), "profiles": profs, "documented": ev in doc_events}
    hierarchy = {c: v.split() for c, v in tsv(od / "hierarchy.tsv")}
    cat_events = {c: v.split() for c, v in tsv(od / "cat_events.tsv")}
    nsmap = {n: v.split() for n, v in tsv(od / "nsmap.tsv")}
    disabled = open(od / "disabled.tsv").read().split()

    per_cmd = collections.defaultdict(lambda: {"valid": [], "invalid": [], "invalid-warn": [],
                                               "profiles": collections.defaultdict(list), "licence": set(),
                                               "proxy": set()})
    oracle_v = {}
    for ev, cmd, verdict, prof, lic, proxy in tsv(od / "oracle.tsv"):
        r = per_cmd[cmd]
        r[verdict].append(ev)
        oracle_v[(ev, cmd)] = verdict
        if prof and verdict != "invalid":
            r["profiles"][prof].append(ev)
        r["licence"].update(lic.split())
        if proxy:
            r["proxy"].add(proxy)

    suspend = {}
    if a.suspend:
        for row in tsv(a.suspend):
            if len(row) >= 3 and row[2] not in ("", "?"):
                suspend[row[0]] = {"depth": int(row[2]), "via": row[3] if len(row) > 3 else "",
                                   "confidence": row[4] if len(row) > 4 else ""}

    commands = {}
    names = set(grammar["commands"]) | set(per_cmd) | set(doc_cmds)
    for name in sorted(names):
        g = grammar["commands"].get(name, {})
        r = per_cmd.get(name)
        commands[name] = {
            "documented": name in doc_cmds,
            "in_checker": name in grammar["commands"] or name in per_cmd,
            "in_tmm": name in tmm,
            "signature": g.get("signature", ""),
            "forms": g.get("forms", []),
            "constraints": g.get("constraints", ""),
            "semantic_checker": g.get("semantic", ""),
            "object_dependencies": sorted({d.split()[-1] for d in g.get("object_dependencies", "").split(" ; ") if d}),
            "value_checks": g.get("value_checks", ""),
            "licence": sorted(r["licence"]) if r else [],
            "proxy": sorted(r["proxy"]) if r else [],
            "events": ({"valid": sorted(r["valid"]), "invalid": sorted(r["invalid"]),
                        "warn_only": sorted(r["invalid-warn"])} if r else {}),
            "profile_requirements": ({k: sorted(v) for k, v in r["profiles"].items()} if r else {}),
            "suspends": suspend.get(name),
            "disabled_in_irules": name in disabled,
        }

    agreement = None
    if a.measured:
        m = json.load(open(a.measured))
        mm = m.get("matrix")
        if mm is None:  # compact form: indices of invalid / warn per event
            cmds = m["commands"]
            mm = {ev: {c: "valid" for c in cmds} for ev in m["events"]}
            for ev, idx in m["invalid"].items():
                for i in idx:
                    mm[ev][cmds[i]] = "invalid"
            for ev, idx in m["invalid_warn"].items():
                for i in idx:
                    mm[ev][cmds[i]] = "invalid-warn"
        n = agree = 0
        dis = []
        for ev, row in mm.items():
            for c, v in row.items():
                n += 1
                if oracle_v.get((ev, c)) == v:
                    agree += 1
                else:
                    dis.append([ev, c, v, oracle_v.get((ev, c))])
        agreement = {"cells": n, "agree": agree, "disagreements": dis}

    model = {
        "events": events,
        "event_category_hierarchy": hierarchy,
        "category_events": cat_events,
        "namespace_event_categories": nsmap,
        "disabled_commands": disabled,
        "commands": commands,
        "tmm_only_commands": sorted(set(tmm) - names),
        "oracle_agreement_with_measured_loader": agreement,
    }
    json.dump(model, open(a.out, "w"), indent=1, sort_keys=False)
    print(f"{len(events)} events, {len(commands)} commands, {len(model['tmm_only_commands'])} TMM-only names"
          + (f", oracle agrees on {agreement['agree']}/{agreement['cells']} measured cells" if agreement else ""))


if __name__ == "__main__":
    main()
