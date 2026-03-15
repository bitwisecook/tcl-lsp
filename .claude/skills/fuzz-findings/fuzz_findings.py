#!/usr/bin/env python3
"""Fuzz findings management tool.

Query, triage, verify, and update differential-fuzzer findings.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

FINDINGS_DIR = Path(__file__).resolve().parent.parent.parent.parent / "tests" / "fuzz" / "findings"


def _load_findings() -> list[dict]:
    """Load all finding JSON files."""
    findings = []
    for f in sorted(FINDINGS_DIR.iterdir()):
        if f.suffix != ".json":
            continue
        data = json.loads(f.read_text())
        data["_file"] = f.name
        findings.append(data)
    return findings


def cmd_summary(args: list[str]) -> None:
    """Print summary of all findings by category and status."""
    findings = _load_findings()
    fixed = [f for f in findings if f.get("fixed")]
    unfixed = [f for f in findings if not f.get("fixed")]

    print(f"Total findings: {len(findings)}")
    print(f"Fixed: {len(fixed)}")
    print(f"Unfixed: {len(unfixed)}")

    # By category
    cats: dict[str, dict[str, int]] = {}
    for f in findings:
        cat = f.get("category", "unknown")
        if cat not in cats:
            cats[cat] = {"fixed": 0, "unfixed": 0}
        if f.get("fixed"):
            cats[cat]["fixed"] += 1
        else:
            cats[cat]["unfixed"] += 1

    print(f"\n{'Category':<30} {'Fixed':>6} {'Unfixed':>8} {'Total':>6}")
    print("-" * 54)
    for cat in sorted(cats, key=lambda c: cats[c]["unfixed"], reverse=True):
        c = cats[cat]
        total = c["fixed"] + c["unfixed"]
        print(f"{cat:<30} {c['fixed']:>6} {c['unfixed']:>8} {total:>6}")


def cmd_list(args: list[str]) -> None:
    """List findings filtered by status and/or category.

    Usage: list [--fixed | --unfixed] [--category CAT] [--limit N]
    """
    show_fixed = None
    category = None
    limit = 50

    i = 0
    while i < len(args):
        if args[i] == "--fixed":
            show_fixed = True
        elif args[i] == "--unfixed":
            show_fixed = False
        elif args[i] == "--category" and i + 1 < len(args):
            i += 1
            category = args[i]
        elif args[i] == "--limit" and i + 1 < len(args):
            i += 1
            limit = int(args[i])
        i += 1

    findings = _load_findings()
    if show_fixed is not None:
        findings = [f for f in findings if f.get("fixed", False) == show_fixed]
    if category:
        findings = [f for f in findings if f.get("category", "unknown") == category]

    print(f"{'Seed':<20} {'Category':<30} {'Status':<8} {'Mismatches'}")
    print("-" * 80)
    for f in findings[:limit]:
        seed = f.get("seed", "?")
        cat = f.get("category", "unknown")
        status = "FIXED" if f.get("fixed") else "OPEN"
        mm_summary = ""
        if f.get("mismatches"):
            m = f["mismatches"][0]
            mm_summary = f"{m.get('detail_a', '')[:40]} vs {m.get('detail_b', '')[:40]}"
        print(f"{seed:<20} {cat:<30} {status:<8} {mm_summary}")

    if len(findings) > limit:
        print(f"\n... {len(findings) - limit} more (use --limit to show more)")


def cmd_show(args: list[str]) -> None:
    """Show details of a specific finding by seed number.

    Usage: show SEED
    """
    if not args:
        print("Usage: show SEED", file=sys.stderr)
        sys.exit(1)

    seed = args[0]
    # Find matching file
    for f in FINDINGS_DIR.iterdir():
        if f.suffix == ".json" and seed in f.stem:
            data = json.loads(f.read_text())
            print(json.dumps(data, indent=2))

            # Also show the TCL script if it exists
            tcl_file = f.with_suffix(".tcl")
            if tcl_file.exists():
                script = tcl_file.read_text()
                print(
                    f"\n--- TCL script ({len(script)} chars, {len(script.splitlines())} lines) ---"
                )
                # Show first 50 lines
                lines = script.splitlines()
                for i, line in enumerate(lines[:50]):
                    print(f"  {i + 1:4d}  {line}")
                if len(lines) > 50:
                    print(f"  ... {len(lines) - 50} more lines")
            return

    print(f"Finding not found: {seed}", file=sys.stderr)
    sys.exit(1)


def cmd_mark_fixed(args: list[str]) -> None:
    """Mark a finding as fixed.

    Usage: mark-fixed SEED [--fix "description"] [--category CAT]
    """
    if not args:
        print("Usage: mark-fixed SEED [--fix DESC] [--category CAT]", file=sys.stderr)
        sys.exit(1)

    seed = args[0]
    fix_desc = None
    category = None

    i = 1
    while i < len(args):
        if args[i] == "--fix" and i + 1 < len(args):
            i += 1
            fix_desc = args[i]
        elif args[i] == "--category" and i + 1 < len(args):
            i += 1
            category = args[i]
        i += 1

    for f in FINDINGS_DIR.iterdir():
        if f.suffix == ".json" and seed in f.stem:
            data = json.loads(f.read_text())
            data["fixed"] = True
            if fix_desc:
                data["fix"] = fix_desc
            if category:
                data["category"] = category
            f.write_text(json.dumps(data, indent=2) + "\n")
            print(f"Marked {f.name} as fixed")
            return

    print(f"Finding not found: {seed}", file=sys.stderr)
    sys.exit(1)


def cmd_mark_unfixed(args: list[str]) -> None:
    """Mark a finding as unfixed (reopen).

    Usage: mark-unfixed SEED
    """
    if not args:
        print("Usage: mark-unfixed SEED", file=sys.stderr)
        sys.exit(1)

    seed = args[0]
    for f in FINDINGS_DIR.iterdir():
        if f.suffix == ".json" and seed in f.stem:
            data = json.loads(f.read_text())
            data["fixed"] = False
            data.pop("fix", None)
            f.write_text(json.dumps(data, indent=2) + "\n")
            print(f"Marked {f.name} as unfixed")
            return

    print(f"Finding not found: {seed}", file=sys.stderr)
    sys.exit(1)


def cmd_categorize(args: list[str]) -> None:
    """Set the category of a finding.

    Usage: categorize SEED CATEGORY
    """
    if len(args) < 2:
        print("Usage: categorize SEED CATEGORY", file=sys.stderr)
        sys.exit(1)

    seed, category = args[0], args[1]
    for f in FINDINGS_DIR.iterdir():
        if f.suffix == ".json" and seed in f.stem:
            data = json.loads(f.read_text())
            data["category"] = category
            f.write_text(json.dumps(data, indent=2) + "\n")
            print(f"Set {f.name} category to '{category}'")
            return

    print(f"Finding not found: {seed}", file=sys.stderr)
    sys.exit(1)


def cmd_verify(args: list[str]) -> None:
    """Run findings through the VM to verify their current status.

    Usage: verify [--unfixed | --fixed | --all] [--category CAT] [--timeout SECS] [--limit N]

    Runs each finding's .tcl file through both vm and vm_opt backends.
    Reports which findings still timeout/crash vs now pass.
    """
    from tests.fuzz.harness import _run_vm

    show = "unfixed"
    category = None
    timeout = 3.0
    limit = 0

    i = 0
    while i < len(args):
        if args[i] == "--unfixed":
            show = "unfixed"
        elif args[i] == "--fixed":
            show = "fixed"
        elif args[i] == "--all":
            show = "all"
        elif args[i] == "--category" and i + 1 < len(args):
            i += 1
            category = args[i]
        elif args[i] == "--timeout" and i + 1 < len(args):
            i += 1
            timeout = float(args[i])
        elif args[i] == "--limit" and i + 1 < len(args):
            i += 1
            limit = int(args[i])
        i += 1

    findings = _load_findings()
    if show == "unfixed":
        findings = [f for f in findings if not f.get("fixed")]
    elif show == "fixed":
        findings = [f for f in findings if f.get("fixed")]
    if category:
        findings = [f for f in findings if f.get("category", "unknown") == category]
    if limit:
        findings = findings[:limit]

    still_broken = 0
    now_ok = 0
    regressions = 0
    results_by_cat: dict[str, dict[str, int]] = {}

    print(f"Verifying {len(findings)} findings (timeout={timeout}s)...\n")

    for f in findings:
        seed = f.get("seed", "?")
        cat = f.get("category", "unknown")
        was_fixed = f.get("fixed", False)
        tcl_file = FINDINGS_DIR / f["_file"].replace(".json", ".tcl")

        if not tcl_file.exists():
            continue

        script = tcl_file.read_text()
        r_vm = _run_vm(script, optimise=False, timeout=timeout)
        r_opt = _run_vm(script, optimise=True, timeout=timeout)

        vm_timeout = r_vm.error_message == "TIMEOUT"
        opt_timeout = r_opt.error_message == "TIMEOUT"
        vm_crash = r_vm.return_code == 2 and not vm_timeout
        opt_crash = r_opt.return_code == 2 and not opt_timeout

        has_problem = vm_timeout or opt_timeout or vm_crash or opt_crash

        if cat not in results_by_cat:
            results_by_cat[cat] = {"broken": 0, "ok": 0, "regression": 0}

        if has_problem:
            if was_fixed:
                regressions += 1
                results_by_cat[cat]["regression"] += 1
                status_str = "REGRESSION"
            else:
                still_broken += 1
                results_by_cat[cat]["broken"] += 1
                status_str = "STILL BROKEN"
            detail = []
            if vm_timeout:
                detail.append("vm:TIMEOUT")
            elif vm_crash:
                detail.append(f"vm:CRASH({r_vm.error_message[:40]})")
            else:
                detail.append(f"vm:{r_vm.error_message and r_vm.error_message[:30] or 'ok'}")
            if opt_timeout:
                detail.append("opt:TIMEOUT")
            elif opt_crash:
                detail.append(f"opt:CRASH({r_opt.error_message[:40]})")
            else:
                detail.append(f"opt:{r_opt.error_message and r_opt.error_message[:30] or 'ok'}")
            print(f"  {status_str:<16} seed_{seed} ({cat}) {' | '.join(detail)}")
        else:
            now_ok += 1
            results_by_cat[cat]["ok"] += 1
            if not was_fixed:
                print(f"  NOW FIXED      seed_{seed} ({cat})")

    print(f"\n{'=' * 60}")
    print(f"Results: {now_ok} ok, {still_broken} still broken, {regressions} regressions")
    print(f"\n{'Category':<30} {'OK':>4} {'Broken':>7} {'Regress':>8}")
    print("-" * 52)
    for cat in sorted(results_by_cat, key=lambda c: results_by_cat[c]["broken"], reverse=True):
        r = results_by_cat[cat]
        print(f"{cat:<30} {r['ok']:>4} {r['broken']:>7} {r['regression']:>8}")


def cmd_batch_mark(args: list[str]) -> None:
    """Mark multiple findings as fixed based on verification results.

    Usage: batch-mark --category CAT --fix "description" [--timeout SECS]

    Runs verification on unfixed findings in the category and marks
    those that no longer timeout/crash as fixed.
    """
    from tests.fuzz.harness import _run_vm

    category = None
    fix_desc = None
    timeout = 3.0

    i = 0
    while i < len(args):
        if args[i] == "--category" and i + 1 < len(args):
            i += 1
            category = args[i]
        elif args[i] == "--fix" and i + 1 < len(args):
            i += 1
            fix_desc = args[i]
        elif args[i] == "--timeout" and i + 1 < len(args):
            i += 1
            timeout = float(args[i])
        i += 1

    if not category or not fix_desc:
        print("Usage: batch-mark --category CAT --fix DESC [--timeout SECS]", file=sys.stderr)
        sys.exit(1)

    findings = _load_findings()
    findings = [
        f for f in findings if not f.get("fixed") and f.get("category", "unknown") == category
    ]

    marked = 0
    for f in findings:
        seed = f.get("seed", "?")
        tcl_file = FINDINGS_DIR / f["_file"].replace(".json", ".tcl")
        if not tcl_file.exists():
            continue

        script = tcl_file.read_text()
        r_vm = _run_vm(script, optimise=False, timeout=timeout)
        r_opt = _run_vm(script, optimise=True, timeout=timeout)

        vm_ok = r_vm.error_message != "TIMEOUT" and r_vm.return_code != 2
        opt_ok = r_opt.error_message != "TIMEOUT" and r_opt.return_code != 2

        if vm_ok and opt_ok:
            # Actually mark it
            json_file = FINDINGS_DIR / f["_file"]
            data = json.loads(json_file.read_text())
            data["fixed"] = True
            data["fix"] = fix_desc
            json_file.write_text(json.dumps(data, indent=2) + "\n")
            marked += 1
            print(f"  MARKED FIXED: seed_{seed}")

    print(f"\nMarked {marked} of {len(findings)} findings as fixed")


def cmd_run(args: list[str]) -> None:
    """Run a specific finding's TCL script through the VM.

    Usage: run SEED [--timeout SECS] [--optimise]
    """
    if not args:
        print("Usage: run SEED [--timeout SECS] [--optimise]", file=sys.stderr)
        sys.exit(1)

    from tests.fuzz.harness import _run_vm

    seed = args[0]
    timeout = 5.0
    optimise = False

    i = 1
    while i < len(args):
        if args[i] == "--timeout" and i + 1 < len(args):
            i += 1
            timeout = float(args[i])
        elif args[i] == "--optimise":
            optimise = True
        i += 1

    for f in FINDINGS_DIR.iterdir():
        if f.suffix == ".tcl" and seed in f.stem:
            script = f.read_text()
            print(
                f"Running {f.name} ({len(script)} chars, optimise={optimise}, timeout={timeout}s)"
            )

            r = _run_vm(script, optimise=optimise, timeout=timeout)
            print(f"\nReturn code: {r.return_code}")
            if r.error_message:
                print(f"Error: {r.error_message}")
            if r.stdout:
                print(f"Stdout: {r.stdout[:500]}")
            return

    print(f"Finding not found: {seed}", file=sys.stderr)
    sys.exit(1)


def main() -> None:
    commands = {
        "summary": cmd_summary,
        "list": cmd_list,
        "show": cmd_show,
        "mark-fixed": cmd_mark_fixed,
        "mark-unfixed": cmd_mark_unfixed,
        "categorize": cmd_categorize,
        "verify": cmd_verify,
        "batch-mark": cmd_batch_mark,
        "run": cmd_run,
    }

    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        print("Fuzz findings management tool\n")
        print("Commands:")
        for name, fn in commands.items():
            doc = (fn.__doc__ or "").strip().split("\n")[0]
            print(f"  {name:<16} {doc}")
        print(f"\nUsage: python3 {sys.argv[0]} <command> [args...]")
        sys.exit(0)

    cmd = sys.argv[1]
    if cmd not in commands:
        print(f"Unknown command: {cmd}", file=sys.stderr)
        print(f"Available: {', '.join(commands)}", file=sys.stderr)
        sys.exit(1)

    commands[cmd](sys.argv[2:])


if __name__ == "__main__":
    main()
