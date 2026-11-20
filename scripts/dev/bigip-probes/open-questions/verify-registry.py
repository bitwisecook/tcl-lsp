#!/usr/bin/env python3
"""Compare tcl-lsp's iRules registry with the BIG-IP checker-derived model.

Inputs (from one BIG-IP build; the model files stay outside the repository):
  --model             model.json from build-irule-model.py
  --grammar           grammar.json from render-irule-grammar.tcl
  --registry          output of `cargo run -p tcl-registry --example dump_irules_surface`
                      fed with every name in the model
  --registry-events   output of `dump_specs meta-events-props`
  --registry-specs    output of `dump_specs irules`
  --out               JSON report (the text summary goes to stdout)

Event legality is compared against the checker's load-time verdict. The
registry folds profile requirements into legality; the checker records them
as attach-time dependencies. Disagreements are therefore split into those a
checker-side profile requirement explains and those it does not.
"""
import argparse
import collections
import json
import re


def jl(path):
    return [json.loads(l) for l in open(path) if l.strip()]


def profsets(req):
    """'{HTTP FASTHTTP}' / '{{CLIENTSSL SERVERSSL PERSIST}}' -> set of names"""
    return set(re.findall(r"[A-Z][A-Z0-9_]+", req))


def main():
    ap = argparse.ArgumentParser()
    for a in ("model", "grammar", "registry", "registry-events", "registry-specs", "out"):
        ap.add_argument("--" + a, required=True)
    a = ap.parse_args()
    model = json.load(open(a.model))
    grammar = json.load(open(a.grammar))["commands"]
    reg = {r["name"]: r for r in jl(a.registry)}
    reg_events = {r["name"]: r for r in jl(a.registry_events)}
    reg_specs = {r["name"]: r for r in jl(a.registry_specs)}
    cmds = model["commands"]
    events = model["events"]
    report = {}

    def is_ns_entry(n):
        return grammar.get(n, {}).get("signature", "").startswith("(namespace")

    checker = {n for n, c in cmds.items() if c["in_checker"] and not is_ns_entry(n)}
    documented = {n for n, c in cmds.items() if c["documented"]}
    tmm = {n for n, c in cmds.items() if c["in_tmm"]} | set(model["tmm_only_commands"])
    in_reg = {n for n, r in reg.items() if r["in_registry"]}
    reg_irules_specs = set(reg_specs)

    # 1. coverage
    cov = {
        "checker_commands": len(checker),
        "checker_missing_from_registry": sorted(checker - in_reg),
        "documented_missing_from_registry": sorted(documented - in_reg),
        "registry_irules_specs_unknown_to_checker_docs_tmm": sorted(reg_irules_specs - checker - documented - tmm),
        "tmm_only_names_in_registry": sorted(set(model["tmm_only_commands"]) & in_reg),
        "disabled_commands_resolving_in_registry": sorted(set(model["disabled_commands"]) & in_reg),
    }
    report["coverage"] = cov

    # 2. events
    ck_ev, rg_ev = set(events), set(reg_events)
    ev_rep = {"checker_events": len(ck_ev), "registry_events": len(rg_ev),
              "checker_events_missing_from_registry": sorted(ck_ev - rg_ev),
              "registry_events_unknown_to_checker": sorted(rg_ev - ck_ev),
              "profile_mismatch": [], "side_mismatch": []}
    for e in sorted(ck_ev & rg_ev):
        cp = profsets(events[e]["profiles"])
        rp = set(reg_events[e]["implied_profiles"])
        if cp != rp:
            ev_rep["profile_mismatch"].append([e, sorted(cp), sorted(rp)])
        cats = set(events[e]["categories"])
        ck_server = "SERVERSIDE" in cats
        if ck_server and not reg_events[e]["server_side"]:
            ev_rep["side_mismatch"].append([e, "checker SERVERSIDE", "registry not server_side"])
    report["events"] = ev_rep

    # 3. legality
    common_ev = sorted(ck_ev & rg_ev)
    ev_profiles = {e: profsets(events[e]["profiles"]) for e in events}
    per_cmd = {}
    totals = collections.Counter()
    for n in sorted(checker & in_reg):
        c = cmds[n]
        valid = set(c["events"].get("valid", []))
        warn = set(c["events"].get("warn_only", []))
        legal = set(reg[n]["legal_events"])
        preq = {}
        for req, evs in c["profile_requirements"].items():
            for e in evs:
                preq.setdefault(e, set()).update(profsets(req))
        fa, fd, fd_prof, warn_legal, warn_illegal = [], [], [], [], []
        for e in common_ev:
            ck = "valid" if e in valid else ("warn" if e in warn else "invalid")
            rl = e in legal
            if ck == "invalid" and rl:
                fa.append(e)
            elif ck == "valid" and not rl:
                # explained if the checker attaches a profile requirement the
                # event itself does not carry
                if e in preq and not (preq[e] & ev_profiles.get(e, set())):
                    fd_prof.append(e)
                else:
                    fd.append(e)
            elif ck == "warn":
                (warn_legal if rl else warn_illegal).append(e)
        totals["cells"] += len(common_ev)
        totals["registry_allows_checker_rejects"] += len(fa)
        totals["registry_rejects_checker_accepts"] += len(fd)
        totals["registry_rejects_profile_conditional"] += len(fd_prof)
        totals["warn_only_registry_allows"] += len(warn_legal)
        totals["warn_only_registry_rejects"] += len(warn_illegal)
        if fa or fd or fd_prof:
            per_cmd[n] = {"registry_allows_checker_rejects": fa, "registry_rejects_checker_accepts": fd,
                          "registry_rejects_profile_conditional": fd_prof}
    report["legality"] = {"totals": dict(totals), "commands": per_cmd}

    # 4. profiles
    prof_rep = []
    for n in sorted(checker & in_reg):
        ckp = set()
        for req in cmds[n]["profile_requirements"]:
            ckp |= profsets(req)
        er = reg[n]["event_requires"] or {}
        rgp = set(er.get("profiles", []))
        if ckp != rgp:
            prof_rep.append([n, sorted(ckp), sorted(rgp)])
    report["profiles"] = prof_rep

    # 5. arity
    ar_rep = []
    for n in sorted(checker & in_reg):
        g = grammar.get(n, {}).get("arity")
        if not g:
            continue
        r = reg[n]["arity"]
        if g[0] != r[0] or g[1] != r[1]:
            ar_rep.append([n, g, r])
    report["arity"] = ar_rep

    # 6. per-form event constraints
    form_rep = []
    for n in sorted(checker & in_reg):
        fs = grammar.get(n, {}).get("forms", [])
        has_form_cons = any(any("Event" in c or "event" in c for c in f["constraints"]) for f in fs)
        if has_form_cons and not reg[n]["event_forms"]:
            form_rep.append([n, [[f["signature"], f["constraints"]] for f in fs if f["constraints"]]])
    report["form_constraints_missing"] = form_rep

    # 7. also_in claims contradicted by the checker
    also = []
    for n in sorted(checker & in_reg):
        er = reg[n]["event_requires"] or {}
        bad = [e for e in er.get("also_in", []) if e in events and e not in cmds[n]["events"].get("valid", [])
               and e not in cmds[n]["events"].get("warn_only", [])]
        if bad:
            also.append([n, bad])
    report["also_in_contradicted"] = also

    json.dump(report, open(a.out, "w"), indent=1)

    # text summary
    print("== coverage")
    for k, v in cov.items():
        print(f"  {k}: {v if isinstance(v, int) else len(v)}")
    print("== events")
    for k, v in ev_rep.items():
        print(f"  {k}: {v if isinstance(v, int) else len(v)}")
    print("== legality (bare command, common events)")
    for k, v in totals.items():
        print(f"  {k}: {v}")
    print(f"  commands with a disagreement: {len(per_cmd)}")
    print(f"== profile requirement mismatches: {len(prof_rep)}")
    print(f"== arity mismatches: {len(ar_rep)}")
    print(f"== commands whose per-form event constraints the registry lacks: {len(form_rep)}")
    print(f"== also_in entries contradicted by the checker: {len(also)}")


if __name__ == "__main__":
    main()
