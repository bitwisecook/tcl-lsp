#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Write the performance section of RELEASE_NOTES.md for the release being cut.

Why this is a script
--------------------
The section is four image links, and each one embeds the version twice — once
in the release-asset URL, once in the alt text. Hand-editing it means four
chances per release to leave last version's number in a URL, which produces a
page that renders perfectly and shows the *previous* release's graphs. That is
the single worst failure this document has, because nothing about it looks
wrong.

The URLs point at release assets rather than at the committed SVGs under
`scripts/perf/graphs/`: a `raw.githubusercontent.com` link tracks whatever the
branch says today, so last year's notes would silently re-render with this
year's graphs. `.github/workflows/perf.yml` uploads `perf-{memory,cpu,walltime}.svg`
and `perf-summary.md` to the GitHub Release on every `v*` tag, and those bytes
never move again.

Idempotent: run it twice and the second run is a no-op. It replaces any
existing `## Performance…` section rather than appending a second one.

Usage:
    python3 scripts/release/perf_notes.py 2.1.20
    python3 scripts/release/perf_notes.py 2.1.20 --check     # verify, write nothing
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import textwrap
from pathlib import Path
from typing import Sequence

ROOT = Path(__file__).resolve().parents[2]
RESULTS = ROOT / "scripts" / "perf" / "results"
NOTES = ROOT / "RELEASE_NOTES.md"

REPO = "bitwisecook/tcl-lsp"
ASSET_BASE = f"https://github.com/{REPO}/releases/download"

#: Heading the section is written under. Matched by prefix when locating an
#: existing section, so a maintainer may add trailing words to it (the current
#: notes say "across the 2.1 pre-releases") without this script duplicating it.
HEADING = "## Performance"


def semver_key(version: str) -> tuple[int, ...]:
    parts = [
        int("".join(c for c in chunk if c.isdigit()) or 0)
        for chunk in version.replace("-", ".").split(".")
    ]
    return tuple(parts + [0] * (4 - len(parts)))


def measured_versions() -> list[str]:
    return sorted((p.stem for p in RESULTS.glob("*.json")), key=semver_key)


def _series(version: str, versions: Sequence[str]) -> tuple[str, str]:
    """`(label, glob)` naming the measured range.

    A range inside one minor line reads as `2.1` / `2.1.x`. The moment it spans
    minors — which it does as soon as the line promotes to a stable even minor
    and the graphs still carry the odd-minor history — naming either minor
    would be false, so it widens to `2.x`.
    """
    minors = {semver_key(v)[:2] for v in versions}
    if len(minors) == 1:
        ((major, minor),) = minors
        return f"{major}.{minor}", f"{major}.{minor}.x"
    return f"{semver_key(version)[0]}.x", f"{semver_key(version)[0]}.x"


def _is_prerelease(version: str) -> bool:
    """Whether `version` belongs to the pre-release channel.

    Delegated to `prerelease.sh` rather than re-deriving the minor parity here:
    that script is the single source of truth every other pre-release switch
    reads, and a second copy of the rule is a second thing to get out of step.
    """
    out = subprocess.run(
        ["bash", str(Path(__file__).with_name("prerelease.sh")), version],
        capture_output=True,
        text=True,
        check=True,
    )
    return out.stdout.strip() == "true"


def _noun(versions: Sequence[str]) -> str:
    """The noun for the whole measured range: "pre-release" or "release".

    Derived from every version in the range, not from the one being cut. The
    noun is applied to the series as a whole, so a range that mixes channels —
    which it does from the first stable even-minor onward — cannot honestly be
    called a pre-release series. "Release" is the safe superset: a pre-release
    is still a release, so a mixed range reads correctly under it, while a
    wholly pre-release range keeps the narrower and more informative word.
    """
    return "pre-release" if all(map(_is_prerelease, versions)) else "release"


def _load(version: str) -> dict:
    return json.loads((RESULTS / f"{version}.json").read_text(encoding="utf-8"))


def _hosts(versions: Sequence[str]) -> list[tuple[str, str]]:
    """Distinct (cpu, platform) pairs across the graphed runs, first use first.

    Half the committed history was measured on the maintainer's laptop and the
    rest on CI runners. A reader comparing a 2026 CPU-seconds figure with a 2025
    one is comparing hardware unless the notes say so, so the notes say so.
    """
    seen: list[tuple[str, str]] = []
    for version in versions:
        payload = _load(version)
        host = payload.get("host") or {}
        entry = (str(host.get("cpu") or "unknown"), str(payload.get("platform", "?")))
        if entry not in seen:
            seen.append(entry)
    return seen


#: Prose is wrapped to match the rest of RELEASE_NOTES.md. Not cosmetic: an
#: unwrapped paragraph makes every later edit a whole-paragraph diff, which
#: hides what actually changed between one release's notes and the next.
WRAP = 78


def _para(text: str) -> list[str]:
    """One wrapped paragraph, followed by a blank line.

    Neither hyphens nor over-long words are break points: the default wrapper
    splits `…/bitwisecook/tcl-lsp/releases/…` after the hyphen and silently
    turns a release-asset link into two lines of plain text.
    """
    return textwrap.wrap(
        " ".join(text.split()),
        width=WRAP,
        break_on_hyphens=False,
        break_long_words=False,
    ) + [""]


def _gaps(versions: Sequence[str]) -> list[str]:
    """Patch numbers missing from the middle of the measured range.

    A reader who counts the legend entries and finds one short deserves to know
    it was never released, rather than assuming the benchmark lost a version.
    """
    keys = [semver_key(v) for v in versions]
    line = {k[:2] for k in keys}
    if len(line) != 1:  # a range spanning minors is not a simple patch sequence
        return []
    major, minor = line.pop()
    present = {k[2] for k in keys}
    return [
        f"{major}.{minor}.{patch}"
        for patch in range(min(present), max(present))
        if patch not in present
    ]


def build_section(version: str, versions: Sequence[str]) -> str:
    """Render the whole `## Performance…` section for `version`."""
    payload = _load(version)
    scope = str(payload.get("scope", "small"))
    revision = str(payload.get("corpus_revision", ""))
    files = int(payload.get("corpus_files") or 0)
    oldest, newest = versions[0], versions[-1]

    corpus = f"a pinned {files}-file corpus (scope `{scope}`"
    corpus += f", revision `{revision}`)" if revision else ")"

    line, glob = _series(version, versions)
    noun = _noun(versions)

    lines = [f"{HEADING} across the {line} {noun}s", ""]
    lines += _para(
        f"These graphs cover every measured `{glob}` {noun} from `v{oldest}` "
        f"through `v{newest}`, run by `scripts/perf/` against {corpus}. The corpus, "
        "scope, and revision are fixed across the whole series, so the lines are "
        "comparable with each other."
    )

    missing = _gaps(versions)
    if missing:
        listed = ", ".join(f"`v{v}`" for v in missing)
        was = "is no" if len(missing) == 1 else "are no"
        lines += _para(
            f"There {was} {listed} point: "
            + ("that version was" if len(missing) == 1 else "those versions were")
            + " never released."
        )

    if version == "2.1.19":
        # v2.1.19's immutable release assets predate the highlighted renderer.
        # Its line charts use elapsed benchmark time on the x-axis, so only the
        # wall-time bars are ordered by release and have a rightmost v2.1.19 bar.
        lines += _para(
            f"**`{version}` is this release**: in the wall-time graph, it is the "
            "rightmost bar in each group. In the memory and CPU graphs, identify "
            "its line using the graph legend; these immutable assets use "
            "categorical colours and dash patterns to distinguish the series."
        )
    else:
        lines += _para(
            f"**`{version}` is this release**: it is the bright-blue line in the "
            "memory and CPU graphs and the rightmost bar in each wall-time group. "
            "Earlier releases are drawn in grey and fade with age."
        )

    hosts = _hosts(versions)
    if len(hosts) > 1:
        listed = "; ".join(f"{cpu} ({platform})" for cpu, platform in hosts)
        lines += _para(
            f"The series spans more than one measurement host — {listed}. Wall time "
            "and CPU are properties of the machine as much as of the build, so "
            "compare within a host era rather than reading the boundary as a "
            "product change. Resident memory is far less host-sensitive."
        )

    for slug, title in (
        ("memory", "Resident memory"),
        ("cpu", "CPU utilisation"),
        ("walltime", "Per-check wall time"),
    ):
        lines += [
            f"### {title}",
            "",
            f"![{title} across the {line} {noun}s]"
            f"({ASSET_BASE}/v{version}/perf-{slug}.svg)",
            "",
        ]

    lines += _para(
        f"[Benchmark table and method notes]({ASSET_BASE}/v{version}/perf-summary.md)"
        f" — the raw result JSON is attached to this release as `perf-{version}.json`."
    )
    return "\n".join(lines)


def splice(text: str, section: str) -> str:
    """Replace the existing performance section, or append one.

    Bounded by the next `## ` heading so the rest of the notes are untouched;
    appended at the end when there is no section yet, which is where it has
    always sat.
    """
    match = re.search(
        rf"(?m)^{re.escape(HEADING)}.*?$(?:\n(?!## ).*)*\n*",
        text,
    )
    if match:
        return text[: match.start()] + section + text[match.end() :]
    return text.rstrip("\n") + "\n\n" + section


def check_heading(text: str, version: str) -> str | None:
    """Complain if the notes are not headed with the version being cut."""
    first = text.lstrip().split("\n", 1)[0].strip()
    if first != f"# v{version}":
        return f"RELEASE_NOTES.md starts with {first!r}, expected '# v{version}'"
    return None


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="perf_notes.py",
        description="Write the performance section of RELEASE_NOTES.md.",
    )
    parser.add_argument("version", help="the release being cut, as X.Y.Z")
    parser.add_argument(
        "--notes", type=Path, default=NOTES, help="notes file (default: %(default)s)"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the section is missing or stale; write nothing",
    )
    args = parser.parse_args(argv)

    version = args.version.lstrip("v")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        print(
            f"error: {args.version!r} is not a version (expected X.Y.Z)",
            file=sys.stderr,
        )
        return 2

    versions = measured_versions()
    if version not in versions:
        print(
            f"error: no benchmark result for {version} in {RESULTS}.\n"
            f"       run: scripts/release/perf_release.sh {version}",
            file=sys.stderr,
        )
        return 1
    if versions[-1] != version:
        # Not fatal: re-rendering an older release's notes is legitimate. But a
        # release whose own measurement is not the newest usually means the
        # benchmark was run before the tree was final.
        print(
            f"warning: {version} is not the newest measured version "
            f"({versions[-1]} is)",
            file=sys.stderr,
        )

    text = args.notes.read_text(encoding="utf-8")
    problem = check_heading(text, version)
    updated = splice(text, build_section(version, versions))

    if args.check:
        if problem:
            print(f"error: {problem}", file=sys.stderr)
            return 1
        if updated != text:
            print(
                f"error: the performance section in {args.notes.name} is missing or "
                f"stale for {version}.\n"
                f"       run: python3 scripts/release/perf_notes.py {version}",
                file=sys.stderr,
            )
            return 1
        print(f"{args.notes.name}: performance section is current for {version}")
        return 0

    if problem:
        # A warning, not a failure: the prose is written before this runs, and
        # refusing to write the graphs because the title is not yet updated
        # would be an unhelpful order to impose.
        print(f"warning: {problem}", file=sys.stderr)

    if updated == text:
        print(f"{args.notes.name}: performance section already current for {version}")
        return 0
    args.notes.write_text(updated, encoding="utf-8", newline="\n")
    print(f"{args.notes.name}: wrote the performance section for {version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
