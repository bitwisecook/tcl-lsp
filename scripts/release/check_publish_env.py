#!/usr/bin/env python3
"""check_publish_env.py — enforce the publish-secret invariant.

Invariant (see docs/design/contracts/release-and-publish.md and AGENTS.md):
a workflow job that references any ``secrets.*`` must bind to a protected,
manually-approved ``environment:``.  That is what keeps a publish token
reachable only by an approval-gated job — never by every workflow run.

Two layers are enforced:

  1. *Presence* — every secret-using job declares an ``environment:`` at all.
  2. *Identity* — the known publish secrets bind to their **specific**
     designated environment (``REQUIRED_ENV`` below), not merely to some
     four-space ``environment:`` key.  Without this, a typo'd or arbitrary
     environment name (which GitHub silently auto-creates as *unprotected*)
     would satisfy layer 1 while defeating the gate, and a token could be
     cross-wired to the wrong protected environment.

This is a dependency-free, indentation-based scan of ``.github/workflows/``
(the repo's Python env has no PyYAML).  It treats a two-space-indented key
under ``jobs:`` as a job, and inspects each job block for ``secrets.``
references and the job's ``environment:`` binding (inline or ``name:`` block).

Exit codes:
  0  every secret-using job declares an environment, and every known publish
     secret binds to its required environment
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
ENV_RE = re.compile(r"^    environment:\s*(?P<inline>\S.*)?$")
SECRET_RE = re.compile(r"secrets\.(?P<name>[A-Za-z_][A-Za-z0-9_]*)")

# The publish secrets that MUST bind to a specific protected Environment.
# Keep in lockstep with the publish jobs in .github/workflows/ci.yml.
REQUIRED_ENV = {
    "VSCE_PAT": "marketplace-vscode",
    "JETBRAINS_TOKEN": "marketplace-jetbrains",
}


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


def job_environment(body: list[str]) -> str | None:
    """Return the environment name a job binds to, or None if it declares none.

    Handles both the inline form ``environment: NAME`` and the block form::

        environment:
          name: NAME
          url: ...
    """
    for i, line in enumerate(body):
        m = ENV_RE.match(line)
        if not m:
            continue
        inline = m.group("inline")
        if inline:
            return inline.strip().strip("'\"")
        # Block form: scan the lines nested under `environment:` (indented
        # deeper than its 4 spaces) for the `name:` key; stop at the dedent.
        for nxt in body[i + 1 :]:
            if not nxt.strip():
                continue
            if len(nxt) - len(nxt.lstrip(" ")) <= 4:
                break
            nm = re.match(r"\s*name:\s*(?P<name>\S+)", nxt)
            if nm:
                return nm.group("name").strip("'\"")
        return None
    return None


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
            env = job_environment(body)
            if env is None:
                violations.append(
                    f"{wf.name}: job '{name}' references {', '.join(secrets)} "
                    f"but declares no `environment:` (publish secrets must be "
                    f"gated by a protected Environment)"
                )
                continue
            # Identity layer: any known publish secret must bind to its own
            # designated environment, not just to *an* environment.
            for secret in secrets:
                want = REQUIRED_ENV.get(secret)
                if want is not None and env != want:
                    violations.append(
                        f"{wf.name}: job '{name}' references {secret} but binds "
                        f"to environment '{env}', not the required '{want}' "
                        f"(a token must reach only its own protected Environment)"
                    )

    if violations:
        print("FAIL: publish-secret invariant violated:", file=sys.stderr)
        for v in violations:
            print(f"  - {v}", file=sys.stderr)
        return 1

    print(
        f"OK: {checked} secret-using job(s) all bind to an environment, "
        f"and every known publish secret targets its required Environment."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
