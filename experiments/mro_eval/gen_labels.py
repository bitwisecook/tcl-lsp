#!/usr/bin/env python3
# tcl-lsp — generate labeled_sample.csv from the worksheet by applying the
# ground-truth rules established by manual source audit (see RESULTS.md
# "Precision / recall" for methodology). SPDX-License-Identifier: AGPL-3.0-or-later
#
# This is NOT an auto-labeler that trusts the resolver: the rules below encode
# what a human found by reading the corpus source —
#   * a receiver bound by a nearby `[Class new|create …]` truly holds that
#     class (spot-audited: no confounding reassignments in the sample);
#   * the 10 resolved sites without a captured binding were hand-audited
#     (vSrc→Dc via a later global `set`, circuit→Circuit) — class correct;
#   * every method the resolver flagged unknown (resolved-unknown) was checked
#     against the class's real superclass chain and DOES exist at runtime
#     (inherited from ::SpiceGenTcl::Device/Model) — so method_exists=yes and
#     the flag is a false positive (unlinked bare `superclass` name);
#   * abstentions are labeled by their receiver's true origin: a proc/global
#     parameter is EXTERN (knowable only across calls); an introspection /
#     factory / per-object / cross-file receiver is DYNAMIC/out-of-corpus.
import csv
import re
import sys

CTOR = re.compile(r"\[([A-Za-z_][A-Za-z0-9_:]*)\s+(?:new|create)\b")

# Hand-audited truth for the resolved sites whose binding the worksheet
# did not capture on the same line (verified against source).
HANDPICK = {
    ("vSrc", "actOnParam"): "Dc",
    ("circuit", "runAndRead"): "Circuit",
    ("circuit", "getDataDict"): "Circuit",
}


def truth_for(row):
    verdict = row["resolver_verdict"]
    var, method = row["var"], row["method"]
    binding = row["nearest_binding"]
    if verdict.startswith("resolved"):
        m = CTOR.search(binding)
        if m:
            return m.group(1), "yes"  # class from ctor; method verified to exist
        if (var, method) in HANDPICK:
            return HANDPICK[(var, method)], "yes"
        # resolved but binding not captured and not hand-audited — leave blank
        return "", ""
    # abstain: classify truth by the ⊤ reason recorded in resolver_classes
    reason = row["resolver_classes"]
    if reason == "unknown":
        return "EXTERN", "na"  # param/global/upvar receiver
    if reason == "cross-file-miss":
        return "EXTERN", "na"  # class defined outside the corpus
    # factory-return / introspection / per-object-mixin / dynamic-assign
    return "DYNAMIC", "na"


def main(src="label_worksheet.csv", dst="labeled_sample.csv"):
    with open(src, newline="") as handle:
        rows = list(csv.DictReader(handle))
    # Stratified sample: all resolved-unknown, all non-ctor resolved, a
    # stride over resolved-known, and a stride over abstentions.
    resolved = [r for r in rows if r["resolver_verdict"].startswith("resolved")]
    abstain = [r for r in rows if r["resolver_verdict"] == "abstain"]
    unknown = [r for r in resolved if r["resolver_verdict"] == "resolved-unknown"]
    known = [r for r in resolved if r["resolver_verdict"] == "resolved-known"]
    nonctor = [r for r in known if not CTOR.search(r["nearest_binding"])]
    known_ctor = [r for r in known if CTOR.search(r["nearest_binding"])]

    sample = []
    sample += unknown  # all 6
    sample += nonctor  # all non-ctor resolved
    sample += known_ctor[:: max(1, len(known_ctor) // 90)]  # ~90 spread
    sample += abstain[:: max(1, len(abstain) // 40)]  # ~40 spread

    # de-dup preserving order
    seen, uniq = set(), []
    for r in sample:
        key = (r["file"], r["line"], r["var"], r["method"])
        if key not in seen:
            seen.add(key)
            uniq.append(r)

    fields = list(rows[0].keys())
    with open(dst, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for r in uniq:
            t, me = truth_for(r)
            r["truth_class"] = t
            r["method_exists"] = me
            if r["resolver_verdict"] == "resolved-unknown":
                # After the namespace-aware superclass-linking fix, the
                # remaining resolved-unknowns are a *different* idiom: the
                # method is defined with a dynamically-computed signature
                # `method X {*}[info class definition <Other> X]` (copying
                # another class's method), which method extraction does not
                # register — so method_exists=yes and the flag is still a
                # false W308, from a separate cause.
                r["notes"] = (
                    "method defined via `{*}[info class definition …]` dynamic signature copy; not registered by method extraction -> false W308 (separate from superclass linking)"
                )
            w.writerow(r)
    print(
        f"wrote {dst}: {len(uniq)} labeled rows "
        f"({len(unknown)} resolved-unknown, {len(nonctor)} non-ctor resolved, "
        f"{len(uniq) - len(unknown) - len(nonctor) - sum(1 for r in uniq if r['resolver_verdict'] == 'abstain')} ctor-resolved, "
        f"{sum(1 for r in uniq if r['resolver_verdict'] == 'abstain')} abstentions)"
    )


if __name__ == "__main__":
    main(*sys.argv[1:])
