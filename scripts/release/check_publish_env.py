#!/usr/bin/env python3
"""check_publish_env.py — enforce the publish-secret invariant.

Invariant (see docs/design/contracts/release-and-publish.md and AGENTS.md):
a workflow job that references any ``secrets.*`` must bind to a protected,
manually-approved ``environment:``.  That is what keeps a publish token
reachable only by an approval-gated job — never by every workflow run.

This is a dependency-free, indentation-based scan of ``.github/workflows/``
(the repo's Python env has no PyYAML).  It treats a two-space-indented key
under ``jobs:`` as a job, and checks each job block for a ``secrets.``
reference and an ``environment:`` key.

Exit codes:
  0  every secret-using job declares an environment
  1  a violation was found
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

WORKFLOWS = Path(__file__).resolve().parents[2] / ".github" / "workflows"

# A job header: exactly two spaces, then `name:` (jobs are 2-indented).
JOB_RE = re.compile(r"^  (?P<name>[A-Za-z0-9_.-]+):\s*$")
# `environment:` as a job key (4 spaces), value inline or as a block.
ENV_RE = re.compile(r"^    environment:")
SECRET_RE = re.compile(r"secrets\.[A-Za-z_][A-Za-z0-9_]*")


def job_blocks(text: str):
    """Yield (job_name, body_lines) for each job under the top-level jobs:."""
    lines = text.splitlines()
    in_jobs = False
    current = None
    body: list[str] = []
    for line in lines:
        if re.match(r"^jobs:\s*$", line):
            in_jobs = True
            continue
        if not in_jobs:
            continue
        # A non-indented, non-blank line ends the jobs: section.
        if line and not line.startswith(" "):
            if current is not None:
                yield current, body
            return
        m = JOB_RE.match(line)
        if m:
            if current is not None:
                yield current, body
            current, body = m.group("name"), []
            continue
        if current is not None:
            body.append(line)
    if current is not None:
        yield current, body


def main() -> int:
    violations: list[str] = []
    checked = 0
    for wf in sorted(WORKFLOWS.glob("*.yml")) + sorted(WORKFLOWS.glob("*.yaml")):
        text = wf.read_text()
        for name, body in job_blocks(text):
            blob = "\n".join(body)
            secrets = sorted(set(SECRET_RE.findall(blob)))
            if not secrets:
                continue
            checked += 1
            has_env = any(ENV_RE.match(b) for b in body)
            if not has_env:
                violations.append(
                    f"{wf.name}: job '{name}' references {', '.join(secrets)} "
                    f"but declares no `environment:` (publish secrets must be "
                    f"gated by a protected Environment)"
                )

    if violations:
        print("FAIL: publish-secret invariant violated:", file=sys.stderr)
        for v in violations:
            print(f"  - {v}", file=sys.stderr)
        return 1

    print(f"OK: {checked} secret-using job(s) all bind to an environment.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
