#!/usr/bin/env python3
# tcl-lsp — score the class-lattice resolver against hand-labeled ground truth.
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Reads a labeled CSV (label_worksheet.csv enriched with the `truth_class`,
# `method_exists`, and `notes` columns) and computes:
#   * class-resolution precision  — of RESOLVED sites, fraction whose named
#     class set contains the true receiver class (i.e. no false resolution);
#   * recall (locally-knowable)   — of sites whose class a human could pin
#     down from the file, fraction the resolver resolved (didn't abstain);
#   * method-flag accuracy        — of resolved sites, whether the resolver's
#     method_known flag agrees with whether the method actually exists.
#
# Ground-truth columns (hand-filled, see labeled_sample.csv):
#   truth_class    fully/partly-qualified true class, or:
#                  "EXTERN" (receiver is a param/global — knowable only across
#                  calls), "DYNAMIC" (genuinely runtime, unknowable),
#                  "" (unlabeled row — skipped).
#   method_exists  yes | no | na   (does the method resolve on the true class
#                  at runtime, incl. inheritance/builtins)
#
# Usage: python3 score_labels.py labeled_sample.csv
import csv
import sys


def tail(qname: str) -> str:
    return qname.rsplit("::", 1)[-1]


def main(path: str) -> None:
    with open(path, newline="") as handle:
        rows = list(csv.DictReader(handle))
    labeled = [r for r in rows if r.get("truth_class", "").strip()]

    # --- class-resolution precision (resolved rows with a concrete truth) ---
    prec_denom = prec_hit = false_resolutions = 0
    false_rows = []
    for r in labeled:
        if not r["resolver_verdict"].startswith("resolved"):
            continue
        truth = r["truth_class"].strip()
        if truth in ("EXTERN", "DYNAMIC"):
            # resolved a site a human calls extern/dynamic — treat as a hit
            # only if it named a class at all; counted separately below.
            continue
        prec_denom += 1
        named = {tail(c) for c in r["resolver_classes"].split("|") if c}
        if tail(truth) in named:
            prec_hit += 1
        else:
            false_resolutions += 1
            false_rows.append(r)

    # --- recall over locally-knowable sites ---
    # A site is "locally knowable" when truth_class is a concrete class
    # (not EXTERN/DYNAMIC). Recall = resolved / knowable.
    know_denom = know_res = 0
    missed = []
    for r in labeled:
        truth = r["truth_class"].strip()
        if truth in ("EXTERN", "DYNAMIC"):
            continue
        know_denom += 1
        if r["resolver_verdict"].startswith("resolved"):
            know_res += 1
        else:
            missed.append(r)

    # --- method-flag accuracy on resolved rows ---
    m_denom = m_ok = 0
    bad_flag = []
    for r in labeled:
        v = r["resolver_verdict"]
        me = r.get("method_exists", "").strip().lower()
        if not v.startswith("resolved") or me not in ("yes", "no"):
            continue
        m_denom += 1
        resolver_says_known = v == "resolved-known"
        truth_known = me == "yes"
        if resolver_says_known == truth_known:
            m_ok += 1
        else:
            bad_flag.append(r)

    # --- correct-abstention audit ---
    abstain_correct = abstain_recallmiss = 0
    for r in labeled:
        if r["resolver_verdict"] != "abstain":
            continue
        truth = r["truth_class"].strip()
        if truth in ("EXTERN", "DYNAMIC"):
            abstain_correct += 1
        else:
            abstain_recallmiss += 1

    def pct(a, b):
        return f"{100.0 * a / b:.1f}%" if b else "n/a"

    print(f"labeled rows: {len(labeled)} (of {len(rows)} worksheet rows)")
    print()
    print("== class-resolution precision (resolved sites, concrete truth) ==")
    print(f"  precision = {prec_hit}/{prec_denom} = {pct(prec_hit, prec_denom)}")
    print(f"  false resolutions (named a WRONG class): {false_resolutions}")
    print()
    print("== recall over locally-knowable sites ==")
    print(f"  recall = {know_res}/{know_denom} = {pct(know_res, know_denom)}")
    print(f"  knowable sites the resolver abstained on: {len(missed)}")
    print()
    print("== method_known flag accuracy (resolved sites) ==")
    print(f"  accuracy = {m_ok}/{m_denom} = {pct(m_ok, m_denom)}")
    print(
        f"  wrong flags (mostly false W308s from unlinked inheritance): {len(bad_flag)}"
    )
    print()
    print("== abstention audit ==")
    print(f"  correct abstentions (truth EXTERN/DYNAMIC): {abstain_correct}")
    print(f"  recall-miss abstentions (truth was knowable): {abstain_recallmiss}")

    if false_rows:
        print("\n-- FALSE RESOLUTIONS --")
        for r in false_rows:
            print(
                f"  {r['file']}:{r['line']} ${r['var']} {r['method']} "
                f"→ {r['resolver_classes']} (truth {r['truth_class']})"
            )
    if bad_flag:
        print("\n-- WRONG method_known FLAGS (candidate false/missed W308) --")
        for r in bad_flag:
            print(
                f"  {r['file']}:{r['line']} ${r['var']} {r['method']} "
                f"→ {r['resolver_verdict']} but method_exists={r['method_exists']}"
                f"  [{r.get('notes', '')}]"
            )


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "labeled_sample.csv")
