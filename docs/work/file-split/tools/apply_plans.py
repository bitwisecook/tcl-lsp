#!/usr/bin/env python3
"""Apply every plan under a directory, parents before the children they create.

    apply_plans.py PLANS_DIR [--root DIR]

A plan whose file does not exist yet waits for the plan that creates it; a
plan whose modules all exist already has been applied and is skipped. Prints
one line per plan and exits non-zero when a plan failed or could not start.
"""

import argparse
import sys
import tomllib
from pathlib import Path

from apply_split import PlanError, child_dir, plan_file


def applied(root: Path, plan: dict) -> bool:
    directory = child_dir(root / plan["file"])
    names = [module["name"] for module in plan.get("module", [])] + plan.get("hoist", [])
    return all((directory / f"{name}.rs").exists() for name in names)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("plans", type=Path)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    arguments = parser.parse_args()
    waiting = {path: tomllib.loads(path.read_text(encoding="utf-8")) for path in sorted(arguments.plans.rglob("*.toml"))}
    failed = False
    progress = True
    while waiting and progress:
        progress = False
        for path, plan in list(waiting.items()):
            if not any((arguments.root / plan[key]).exists() for key in ("file", "move_from") if key in plan):
                continue
            del waiting[path]
            progress = True
            if applied(arguments.root, plan):
                print(f"skip     {path}: already applied")
                continue
            try:
                sizes = plan_file(arguments.root, plan, write=True)
            except PlanError as error:
                print(f"FAILED   {path}: {error}")
                failed = True
                continue
            over = [f"{name} ({lines})" for name, lines in sizes if lines > 2000]
            print(f"applied  {path}: {len(sizes) - 1} modules" + (f"; still over: {', '.join(over)}" if over else ""))
    for path, plan in waiting.items():
        print(f"WAITING  {path}: {plan['file']} does not exist")
    return 1 if failed or waiting else 0


if __name__ == "__main__":
    sys.exit(main())
