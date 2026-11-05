#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Cut a branch holding one commit with a source branch's tree on top of its base.

Generated evidence once pushed `work` history past 10 GB. Deleting it at the
tip does not shrink clones; this tool writes the source tip's tree as a single
commit whose parent is the base tip (when the source already contains it) or
the source/base merge-base. Base history and tags are untouched, and the new
branch's history carries only the tree that exists today.

    cut-squashed-branch.py cut --source origin/work --base origin/rust --branch work-squashed
    cut-squashed-branch.py cut ... --push
    cut-squashed-branch.py audit

`cut` refuses a dirty tree, a tree over the tracked-size budget, and a branch
name that already exists locally or on the remote. It pushes only with --push.
`audit` lists remote-tracking branches whose history touches the bloat paths,
so they can be retired once the squashed branch replaces `work`.
"""

from __future__ import annotations

import argparse
import importlib.util
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

BLOAT_PATHS = (
    "docs/work/name-handling/handoff",
    "docs/work/name-handling/scratchpad-mirror",
    "docs/design/analysis/name-resolution-proofs/rust-validation",
    "docs/design/analysis/name-resolution-proofs/manifest.json",
    "experiments/name-handling/results",
    "experiments/name-handling/f5/answers",
    "experiments/name-handling/patterns/f5/answers",
    "scripts/dev/bigip-probes/resolution-2286/evidence",
)


def _size_budget():
    path = Path(__file__).with_name("check-tracked-size.py")
    spec = importlib.util.spec_from_file_location("check_tracked_size", path)
    assert spec and spec.loader
    module = sys.modules.setdefault(spec.name, importlib.util.module_from_spec(spec))
    spec.loader.exec_module(module)
    return module


class CutError(Exception):
    pass


def git(*args: str, cwd: Path, check: bool = True) -> str:
    result = subprocess.run(
        ["git", *args], cwd=cwd, capture_output=True, text=True, check=False
    )
    if check and result.returncode:
        raise CutError(f"git {' '.join(args)}: {result.stderr.strip()}")
    return result.stdout.strip() if result.returncode == 0 else ""


def is_ancestor(ancestor: str, descendant: str, cwd: Path) -> bool:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        cwd=cwd,
        capture_output=True,
        check=False,
    )
    return result.returncode == 0


@dataclass(frozen=True)
class Plan:
    source: str
    base: str
    parent: str
    tree: str
    commits_squashed: int


def plan(
    root: Path, source: str, base: str, deepen: int = 0, remote: str = "origin"
) -> Plan:
    source_sha = git("rev-parse", "--verify", f"{source}^{{commit}}", cwd=root)
    base_sha = git("rev-parse", "--verify", f"{base}^{{commit}}", cwd=root)
    parent = ""
    for attempt in range(deepen + 1):
        if is_ancestor(base_sha, source_sha, root):
            parent = base_sha
            break
        parent = git("merge-base", source_sha, base_sha, cwd=root, check=False)
        shallow = git("rev-parse", "--is-shallow-repository", cwd=root) == "true"
        if parent or attempt == deepen or not shallow:
            break
        refs = [
            f"+refs/heads/{name[len(remote) + 1 :]}:refs/remotes/{name}"
            for name in (source, base)
            if name.startswith(f"{remote}/")
        ]
        git("fetch", "--filter=blob:none", "--deepen=500", remote, *refs, cwd=root)
    if not parent:
        raise CutError(
            f"no merge-base between {source} and {base}; "
            "fetch more history (e.g. git fetch --filter=blob:none --unshallow) or pass --deepen"
        )
    count = git("rev-list", "--count", f"{parent}..{source_sha}", cwd=root, check=False)
    return Plan(
        source=source_sha,
        base=base_sha,
        parent=parent,
        tree=git("rev-parse", f"{source_sha}^{{tree}}", cwd=root),
        commits_squashed=int(count or 0),
    )


def tree_problems(root: Path, tree: str) -> tuple[int, list[str]]:
    budget = _size_budget()
    allow = budget.read_allowlist(root / budget.ALLOWLIST)
    total = 0
    problems = []
    for line in git("ls-tree", "-r", "-l", tree, cwd=root).splitlines():
        meta, path = line.split("\t", 1)
        size = meta.split()[3]
        if not size.isdigit():
            continue
        total += int(size)
        if int(size) > budget.FILE_LIMIT and path not in allow:
            problems.append(
                f"{path}: {int(size) / budget.MIB:.1f} MiB over the per-file budget"
            )
    if total > budget.TREE_LIMIT:
        problems.append(f"tree is {total / budget.MIB:.0f} MiB, over the tree budget")
    return total, problems


def branch_exists(root: Path, remote: str, branch: str) -> bool:
    if git(
        "rev-parse",
        "--verify",
        "--quiet",
        f"refs/heads/{branch}",
        cwd=root,
        check=False,
    ):
        return True
    return bool(git("ls-remote", "--heads", remote, branch, cwd=root, check=False))


def cut(args: argparse.Namespace) -> int:
    root = args.root
    if git("status", "--porcelain", "--untracked-files=no", cwd=root):
        raise CutError("working tree has uncommitted changes")
    if branch_exists(root, args.remote, args.branch):
        raise CutError(
            f"branch {args.branch} already exists locally or on {args.remote}"
        )
    p = plan(root, args.source, args.base, args.deepen, args.remote)
    total, problems = tree_problems(root, p.tree)
    if problems:
        raise CutError("tree is over budget:\n  " + "\n  ".join(problems))
    message = args.message or (
        f"Squash {args.source} onto {args.base}\n\n"
        f"Tree of {args.source} at {p.source}, replacing {p.commits_squashed} commits "
        f"since {p.parent[:12]}. The original history remains on {args.source} "
        "until it is retired."
    )
    commit = git("commit-tree", p.tree, "-p", p.parent, "-m", message, cwd=root)
    git("branch", args.branch, commit, cwd=root)
    print(f"source   {args.source} {p.source}")
    print(f"parent   {p.parent} ({'base tip' if p.parent == p.base else 'merge-base'})")
    print(
        f"squashed {p.commits_squashed} commits; tree {total / (1024 * 1024):.0f} MiB"
    )
    print(f"branch   {args.branch} -> {commit}")
    if args.push:
        git("push", args.remote, f"{commit}:refs/heads/{args.branch}", cwd=root)
        print(f"pushed   {args.remote}/{args.branch}")
    else:
        print(f"push with: git push {args.remote} {args.branch}")
    return 0


def audit(args: argparse.Namespace) -> int:
    root = args.root
    refs = git(
        "for-each-ref",
        "--format=%(refname:short)",
        f"refs/remotes/{args.remote}",
        cwd=root,
    ).splitlines()
    carrying = []
    for ref in refs:
        if ref.endswith("/HEAD"):
            continue
        hit = git(
            "log",
            "-1",
            "--format=%h %cs",
            ref,
            "--",
            *args.paths,
            cwd=root,
            check=False,
        )
        if hit:
            carrying.append(ref)
            print(f"bloat  {ref}  (last touched {hit})")
    print(f"{len(carrying)} of {len(refs)} remote branches carry bloat history")
    return 1 if carrying and args.strict else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--remote", default="origin")
    sub = parser.add_subparsers(dest="command", required=True)
    c = sub.add_parser("cut", help="create the squashed branch")
    c.add_argument("--source", default="origin/work")
    c.add_argument("--base", default="origin/rust")
    c.add_argument("--branch", required=True)
    c.add_argument("--message")
    c.add_argument(
        "--deepen", type=int, default=0, help="deepen a shallow clone up to N times"
    )
    c.add_argument("--push", action="store_true")
    a = sub.add_parser("audit", help="list remote branches carrying bloat history")
    a.add_argument("--paths", nargs="+", default=list(BLOAT_PATHS))
    a.add_argument(
        "--strict", action="store_true", help="exit 1 if any branch carries bloat"
    )
    args = parser.parse_args(argv)
    try:
        return cut(args) if args.command == "cut" else audit(args)
    except CutError as error:
        print(f"cut-squashed-branch: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
