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

"""Render benchmark result JSON into committable SVG graphs and a release-notes fragment.

Why this exists
---------------
Performance regressions in the language server (a memory leak in the workspace
scanner, a check that quietly went from 0.2s to 8s) are invisible in a table of
numbers spread across releases. Graphing every released version on one set of
axes makes "this version's RSS never plateaus" a shape you notice rather than a
column you have to diff by eye.

Why the output is committed
---------------------------
The SVGs live in the repo and are embedded by URL in release notes, so they must
be reproducible: regenerating from unchanged inputs must produce byte-identical
files. A non-empty `git diff` after a regeneration is then a real signal that the
measurements moved, not noise from a timestamp or a dict that happened to iterate
differently. Everything here is therefore sorted explicitly, formatted with fixed
precision, and free of clocks and random identifiers.

Why the standard library only
-----------------------------
This runs in CI images that only guarantee a CPython interpreter, and pinning a
matplotlib/numpy stack for six line charts is a maintenance cost with no payoff.
The SVG is written as XML text; the charting primitives needed (nice tick steps,
linear/log scales, grouped bars) are a couple of hundred lines.

Why light *and* dark rendering
------------------------------
GitHub renders release notes on both a light and a dark page, and an SVG embedded
via `![](...)` is loaded as an image, so it cannot inherit the page's colours. The
SVG therefore carries its own `@media (prefers-color-scheme: dark)` block and all
chrome (axes, ticks, text) is painted from `currentColor`, which that block flips.

Why one release is blue and the rest are grey
---------------------------------------------
These graphs are read in release notes, where the question is never "how does
2.1.7 compare with 2.1.9" — it is "where does *this* release sit". Twenty
equally-loud categorical colours answer neither: past eight series the palette
wraps, and the reader has to hunt the legend for the line they actually came for.
So the release being cut is drawn in a bright, saturated blue at double stroke
width and painted last (on top of everything), and every earlier release is drawn
on a neutral ramp that fades with age. The result separates by hue, by lightness,
by stroke width and by z-order at once, which survives greyscale printing and
every form of colour-vision deficiency.

`--no-highlight` restores the flat Okabe-Ito categorical palette for the other
job these graphs occasionally do: comparing two arbitrary versions against each
other, where no single series is the subject.

Usage:
    python3 report.py --results scripts/perf/results --out scripts/perf/graphs
    python3 report.py --highlight 2.1.19          # default: the newest result
    python3 report.py --no-log --only 2.1.13,2.1.14,2.1.15
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# --------------------------------------------------------------------------
# Constants
# --------------------------------------------------------------------------

#: Directory this script lives in. Defaults are resolved against it (matching
#: the sibling `bench.py`) so the tool behaves the same from any cwd.
HERE = Path(__file__).resolve().parent

SCHEMA_VERSION = 1

KIB_PER_MIB = 1024.0

# Okabe-Ito: a categorical palette chosen for deuteranopia/protanopia safety.
# Each entry is (light-background colour, dark-background colour). Only the
# final entry differs between the two: pure black vanishes on a dark page, so
# it becomes white there. Keeping the pairs explicit means the swap is data,
# not a special case in the emitter.
#: Stroke patterns layered under the palette once it wraps, so a 16-version
#: sweep does not draw two releases identically. Solid first, so a run with
#: eight or fewer series is unchanged.
DASH_CYCLE: tuple[str, ...] = ("", "7 3", "2 3", "11 3 2 3")

#: Reached only under `--no-highlight`; the default release-notes rendering uses
#: HIGHLIGHT + HISTORY_RAMP below.
PALETTE: tuple[tuple[str, str], ...] = (
    ("#E69F00", "#E69F00"),  # orange
    ("#56B4E9", "#56B4E9"),  # sky blue
    ("#009E73", "#009E73"),  # bluish green
    ("#F0E442", "#F0E442"),  # yellow
    ("#0072B2", "#0072B2"),  # blue
    ("#D55E00", "#D55E00"),  # vermillion
    ("#CC79A7", "#CC79A7"),  # reddish purple
    ("#000000", "#FFFFFF"),  # black / white — the scheme-dependent slot
)

#: The release being cut, as (light-page, dark-page). The one saturated colour
#: on the chart, and the only one that is a hue at all once the history goes
#: grey — so "which line is the new release" is answered without the legend.
#: Two values because a single blue cannot be vivid on both pages: #0A66FF is
#: near-black on a dark ground and #58A6FF washes out on a white one.
HIGHLIGHT: tuple[str, str] = ("#0A66FF", "#58A6FF")

#: Ends of the neutral ramp the *earlier* releases are drawn on, oldest first,
#: as (light-page, dark-page) at each end. Runs arrive semver-ordered, so ramp
#: position is release age: the fan darkens (light page) / brightens (dark page)
#: toward the present, which makes "older" readable off the chart itself rather
#: than only off the legend. Both ends stay well clear of HIGHLIGHT's chroma so
#: no history line can be mistaken for the current one.
HISTORY_RAMP: tuple[tuple[str, str], tuple[str, str]] = (
    ("#C6CCD3", "#444D57"),  # oldest release in the set
    ("#59626C", "#AEB7C2"),  # the release immediately before the current one
)

#: Suffix appended to the highlighted series' legend label. Colour alone is not
#: enough here: a reader who prints the notes, or who has the graph open next to
#: a different release's, needs the chart to say which version it is about.
CURRENT_SUFFIX = " — this release"

# Stroke weights, as the literal CSS text so the emitted bytes are exactly these.
# `.series` is unchanged from the flat-palette rendering, which keeps
# `--no-highlight` output byte-identical to what it always was; the other two
# apply only when a release is highlighted.
SERIES_W = "2"
HISTORY_W = "1.4"  # thin enough that eighteen of them read as one context fan
CURRENT_W = "3.6"  # well over double, so z-order is not the only thing on top

# Chrome colours. Everything structural is drawn in `currentColor`, and these
# two values are what `color` resolves to per scheme.
INK_LIGHT = "#1F2328"
INK_DARK = "#E6EDF3"

# Layout. Fixed so that two runs over the same data cannot differ.
PLOT_W = 780
PLOT_H = 320
MARGIN_LEFT = 78
MARGIN_RIGHT = 28
MARGIN_TOP = 56
AXIS_LABEL_H = 46  # room under the axis for tick labels + axis title
LEGEND_ROW_H = 20
LEGEND_COLS = 4
FOOTNOTE_H = 20


# --------------------------------------------------------------------------
# Input model
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class Check:
    """One measured operation within a run.

    `ok=False` means the check timed out or errored. `wall_s` then holds the
    timeout budget rather than a measurement, which is why callers must never
    aggregate or plot it as if it were a real duration without flagging it.
    """

    ord: int
    id: str
    label: str
    wall_s: float
    cpu_s: float
    rss_before_kib: float
    rss_after_kib: float
    ok: bool
    note: str | None


@dataclass(frozen=True)
class Sample:
    """One point on the resident-memory / CPU sampling timeline.

    `cpu_s` is *cumulative* process CPU time, not an instantaneous rate; it has
    to be differentiated against `t_s` before it means anything as a percentage.
    """

    t_s: float
    rss_kib: float
    cpu_s: float


@dataclass(frozen=True)
class Run:
    """A single released version's benchmark result file."""

    version: str
    platform: str
    corpus_files: int
    # `scope` and `corpus_revision` identify exactly which corpus was measured;
    # `bench.py` emits both, but they are not part of the schema-1 contract, so
    # they are optional and only decorate the summary's provenance line.
    scope: str
    corpus_revision: str
    host_cpu: str
    host_cores: int
    host_mem_gb: int
    checks: tuple[Check, ...]
    timeline: tuple[Sample, ...]

    @property
    def peak_rss_mib(self) -> float:
        return max((s.rss_kib for s in self.timeline), default=0.0) / KIB_PER_MIB

    @property
    def final_rss_mib(self) -> float:
        return (self.timeline[-1].rss_kib / KIB_PER_MIB) if self.timeline else 0.0

    @property
    def first_rss_mib(self) -> float:
        return (self.timeline[0].rss_kib / KIB_PER_MIB) if self.timeline else 0.0

    @property
    def rss_growth_mib(self) -> float:
        """Net resident growth across the run.

        Reported rather than peak alone because a server that allocates and
        releases is fine, while one that ends far above where it started is the
        leak shape this whole report exists to surface.
        """
        return self.final_rss_mib - self.first_rss_mib

    @property
    def total_cpu_s(self) -> float:
        """Whole-process CPU time.

        Taken from the last cumulative timeline sample when there is one: it
        covers background work (indexing threads) that no individual check is
        credited with. Falls back to the per-check sum when no timeline exists.
        """
        if self.timeline:
            return self.timeline[-1].cpu_s
        return sum(c.cpu_s for c in self.checks)

    @property
    def total_wall_s(self) -> float:
        return sum(c.wall_s for c in self.checks)

    @property
    def failed_checks(self) -> int:
        return sum(1 for c in self.checks if not c.ok)


def semver_key(version: str) -> tuple[int, ...]:
    """Sort key ordering versions numerically, so 2.1.9 precedes 2.1.10.

    Lexical ordering is wrong for the release line this tool graphs, and a full
    PEP 440 / semver parser is more than is needed: results files are named for
    plain released versions. Non-numeric components (a `-rc1` suffix) degrade to
    0 so they sort just before the corresponding release rather than throwing.
    """
    parts: list[int] = []
    for chunk in version.replace("-", ".").split("."):
        digits = "".join(ch for ch in chunk if ch.isdigit())
        parts.append(int(digits) if digits else 0)
    # Pad so that "2.1" and "2.1.0" compare equal rather than by length.
    while len(parts) < 4:
        parts.append(0)
    return tuple(parts)


def _as_float(value: Any, default: float = 0.0) -> float:
    """Coerce a JSON scalar to float, tolerating nulls in optional fields."""
    if value is None:
        return default
    return float(value)


def parse_run(payload: dict[str, Any], source: Path) -> Run:
    """Build a `Run` from one decoded results document.

    Validates the schema version up front: silently mis-reading a future schema
    would produce a graph that looks plausible and is wrong, which is worse than
    a hard failure.
    """
    schema = payload.get("schema")
    if schema != SCHEMA_VERSION:
        raise ValueError(
            f"{source}: unsupported schema {schema!r}, expected {SCHEMA_VERSION}"
        )

    host = payload.get("host") or {}
    checks = tuple(
        # Sorted by `ord` here so downstream code never has to care about the
        # order the file happened to list them in.
        sorted(
            (
                Check(
                    ord=int(c.get("ord", 0)),
                    id=str(c["id"]),
                    label=str(c.get("label", c["id"])),
                    wall_s=_as_float(c.get("wall_s")),
                    cpu_s=_as_float(c.get("cpu_s")),
                    rss_before_kib=_as_float(c.get("rss_before_kib")),
                    rss_after_kib=_as_float(c.get("rss_after_kib")),
                    ok=bool(c.get("ok", True)),
                    note=c.get("note"),
                )
                for c in payload.get("checks") or []
            ),
            key=lambda c: (c.ord, c.id),
        )
    )
    timeline = tuple(
        sorted(
            (
                Sample(
                    t_s=_as_float(s.get("t_s")),
                    rss_kib=_as_float(s.get("rss_kib")),
                    cpu_s=_as_float(s.get("cpu_s")),
                )
                for s in payload.get("timeline") or []
            ),
            key=lambda s: s.t_s,
        )
    )
    return Run(
        version=str(payload["version"]),
        platform=str(payload.get("platform", "unknown")),
        corpus_files=int(payload.get("corpus_files") or 0),
        scope=str(payload.get("scope", "")),
        corpus_revision=str(payload.get("corpus_revision", "")),
        # `or` rather than a `get` default throughout: a field that is
        # *present but null* — which is what an unpopulated host probe
        # writes — slips straight past a default and only fails at the
        # int()/str() conversion, which is how the first CI run died.
        host_cpu=str(host.get("cpu") or "unknown"),
        host_cores=int(host.get("cores") or 0),
        host_mem_gb=int(host.get("mem_gb") or 0),
        checks=checks,
        timeline=timeline,
    )


def load_runs(results_dir: Path, only: Sequence[str] | None = None) -> list[Run]:
    """Load every `*.json` under `results_dir`, semver-ordered.

    Files are globbed then sorted by name before parsing purely so that error
    messages come out in a stable order; the returned list is ordered by
    semantic version, which is what fixes the palette assignment per version.
    """
    wanted = set(only) if only else None
    runs: list[Run] = []
    for path in sorted(results_dir.glob("*.json")):
        with path.open(encoding="utf-8") as fh:
            payload = json.load(fh)
        run = parse_run(payload, path)
        if wanted is not None and run.version not in wanted:
            continue
        runs.append(run)
    runs.sort(key=lambda r: semver_key(r.version))
    return runs


def cpu_series(run: Run) -> list[tuple[float, float]]:
    """Differentiate cumulative CPU seconds into instantaneous CPU percent.

    `cpu_pct = 100 * dcpu / dt`. Values above 100 are expected and meaningful —
    the server is multi-threaded, so 350% is three and a half cores busy — and
    are deliberately not clamped. Samples with a non-positive `dt` (a duplicated
    or out-of-order timestamp) are dropped rather than producing an infinity
    that would flatten the whole axis.
    """
    points: list[tuple[float, float]] = []
    for prev, cur in zip(run.timeline, run.timeline[1:]):
        dt = cur.t_s - prev.t_s
        if dt <= 0.0:
            continue
        dcpu = cur.cpu_s - prev.cpu_s
        points.append((cur.t_s, 100.0 * dcpu / dt))
    return points


def rss_series(run: Run) -> list[tuple[float, float]]:
    """Timeline as (elapsed seconds, RSS MiB) pairs."""
    return [(s.t_s, s.rss_kib / KIB_PER_MIB) for s in run.timeline]


# --------------------------------------------------------------------------
# Number and text formatting
# --------------------------------------------------------------------------


def fnum(value: float) -> str:
    """Format a coordinate with fixed precision.

    Every number that reaches the SVG goes through here (or `ftick`). Fixed
    precision is what makes the output byte-stable: `repr()` of a float differs
    with the last bit of an accumulated sum, and 1/100th of a pixel is far below
    anything visible.
    """
    formatted = f"{value:.2f}"
    # Normalise the sign of a rounded-to-zero negative so -0.00 never appears.
    return "0.00" if formatted == "-0.00" else formatted


def ftick(value: float) -> str:
    """Format a tick label: integral values without a decimal point, else 2dp."""
    rounded = round(value, 10)
    if rounded == int(rounded) and abs(rounded) < 1e9:
        return str(int(rounded))
    if abs(rounded) < 0.01:
        return f"{rounded:.4f}"
    return f"{rounded:.2f}"


def esc(text: str) -> str:
    """Escape text for XML content and double-quoted attribute values."""
    return (
        text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


# --------------------------------------------------------------------------
# Scales
# --------------------------------------------------------------------------


def nice_ticks(
    lo: float, hi: float, target: int = 6
) -> tuple[float, float, list[float]]:
    """Choose a rounded (lo, hi, ticks) triple covering [lo, hi] on a linear axis.

    Uses the standard 1/2/5-times-a-power-of-ten step so the labels are numbers a
    reader can hold in their head. Returns the *expanded* bounds, because ending
    an axis mid-step leaves a tick floating off the end.

    Degenerate inputs are the interesting cases: a single-point series gives
    lo == hi, and an empty one gives whatever sentinel the caller passed. Both
    are widened to a non-zero span so nothing downstream divides by zero.
    """
    if not math.isfinite(lo) or not math.isfinite(hi):
        lo, hi = 0.0, 1.0
    if hi < lo:
        lo, hi = hi, lo
    if hi - lo <= 0.0:
        # Widen around the single value: 10% either side, or ±1 at zero.
        pad = abs(hi) * 0.1 or 1.0
        lo, hi = lo - pad, hi + pad

    raw_step = (hi - lo) / max(target, 1)
    magnitude = 10.0 ** math.floor(math.log10(raw_step))
    for multiple in (1.0, 2.0, 2.5, 5.0, 10.0):
        step = magnitude * multiple
        if step >= raw_step:
            break
    lo_t = math.floor(lo / step) * step
    hi_t = math.ceil(hi / step) * step

    ticks: list[float] = []
    # Integer counting rather than repeated addition keeps float drift out of
    # the labels (and so out of the committed bytes).
    count = round((hi_t - lo_t) / step)
    for i in range(count + 1):
        ticks.append(round(lo_t + i * step, 10))
    return lo_t, hi_t, ticks


def log_ticks(lo: float, hi: float) -> tuple[float, float, list[float]]:
    """Choose (lo, hi, ticks) for a log10 axis spanning [lo, hi].

    Bounds snap outward to whole decades. When the data covers only two decades
    or fewer the decades alone would give three labels for the entire axis, so
    the 2x and 5x minor ticks are added — that is the case the wall-time graph
    hits when every check is between 0.05s and 3s.
    """
    lo = lo if lo > 0.0 and math.isfinite(lo) else 1e-3
    hi = hi if hi > lo and math.isfinite(hi) else lo * 10.0

    lo_e = math.floor(math.log10(lo))
    hi_e = math.ceil(math.log10(hi))
    if hi_e <= lo_e:
        hi_e = lo_e + 1

    mantissas = (1.0, 2.0, 5.0) if (hi_e - lo_e) <= 2 else (1.0,)
    ticks: list[float] = []
    for exponent in range(int(lo_e), int(hi_e) + 1):
        for mantissa in mantissas:
            value = round(mantissa * (10.0**exponent), 12)
            if 10.0**lo_e <= value <= 10.0**hi_e:
                ticks.append(value)
    return 10.0**lo_e, 10.0**hi_e, sorted(set(ticks))


def make_scale(
    lo: float, hi: float, pix_lo: float, pix_hi: float, log: bool
) -> Callable[[float], float]:
    """Build a value -> pixel mapping, log10 or linear.

    On a log scale non-positive values cannot be placed at all; they are clamped
    to the axis floor so a zero-duration check renders as a zero-height bar
    instead of raising.
    """
    if log:
        lo_l = math.log10(lo if lo > 0 else 1e-9)
        hi_l = math.log10(hi if hi > 0 else 1.0)
        span = hi_l - lo_l or 1.0

        def scale_log(value: float) -> float:
            v = math.log10(value) if value > 0 else lo_l
            v = min(max(v, lo_l), hi_l)
            return pix_lo + (v - lo_l) / span * (pix_hi - pix_lo)

        return scale_log

    span = (hi - lo) or 1.0

    def scale_lin(value: float) -> float:
        return pix_lo + (value - lo) / span * (pix_hi - pix_lo)

    return scale_lin


# --------------------------------------------------------------------------
# SVG scaffolding
# --------------------------------------------------------------------------


def _rgb(value: str) -> tuple[int, int, int]:
    """Split `#RRGGBB` into integer channels."""
    v = value.lstrip("#")
    return int(v[0:2], 16), int(v[2:4], 16), int(v[4:6], 16)


def _mix(a: str, b: str, t: float) -> str:
    """Blend two `#RRGGBB` colours, `t` running 0.0 (all `a`) to 1.0 (all `b`).

    Rounds by adding a half and truncating rather than calling `round()`, whose
    banker's rounding turns an exact .5 into a value that depends on the channel
    it lands in — a needless way for the committed bytes to become surprising.
    """
    return "#{:02X}{:02X}{:02X}".format(
        *(int(x + (y - x) * t + 0.5) for x, y in zip(_rgb(a), _rgb(b)))
    )


def series_colours(count: int, highlight: int | None) -> list[tuple[str, str]]:
    """Per-series `(light-page, dark-page)` colours, in series order.

    With a `highlight` index — the default, and what the release notes want —
    that series takes `HIGHLIGHT` and every other one is placed on the
    `HISTORY_RAMP` by its position among the remaining series. Because runs are
    semver-ordered, that position is release age.

    With `highlight=None` every series gets a categorical Okabe-Ito colour, for
    comparing arbitrary versions where none of them is *the* subject.
    """
    if highlight is None:
        return [PALETTE[i % len(PALETTE)] for i in range(count)]

    (old_l, old_d), (new_l, new_d) = HISTORY_RAMP
    # Position of each non-highlighted series within the history, precomputed so
    # this stays linear rather than scanning the list once per series.
    history_pos = {
        idx: pos for pos, idx in enumerate(i for i in range(count) if i != highlight)
    }
    span = max(len(history_pos) - 1, 1)
    out: list[tuple[str, str]] = []
    for i in range(count):
        if i == highlight:
            out.append(HIGHLIGHT)
            continue
        # A lone history series sits at the recent end of the ramp rather than
        # the faint end: with one line of context, faint is just hard to read.
        t = history_pos[i] / span if len(history_pos) > 1 else 1.0
        out.append((_mix(old_l, new_l, t), _mix(old_d, new_d, t)))
    return out


def _stylesheet(series_count: int, highlight: int | None) -> str:
    """Per-document CSS: scheme-aware chrome plus one class per series.

    Series colours are emitted as classes rather than inline `stroke=` so the
    per-scheme override (the black palette slot, and every ramp step once a
    release is highlighted) is one rule rather than a per-element branch.
    """
    colours = series_colours(series_count, highlight)
    rules = [
        f":root {{ color: {INK_LIGHT}; }}",
        ".chrome { stroke: currentColor; fill: none; stroke-width: 1; }",
        ".grid { stroke: currentColor; stroke-width: 1; opacity: 0.15; }",
        (
            "text { fill: currentColor; font-family: -apple-system, BlinkMacSystemFont,"
            " 'Segoe UI', Helvetica, Arial, sans-serif; font-size: 11px; }"
        ),
        ".title { font-size: 15px; font-weight: 600; }",
        ".axis-title { font-size: 12px; }",
        ".note { font-size: 10px; opacity: 0.75; }",
        (
            f".series {{ fill: none; stroke-width: {SERIES_W};"
            " stroke-linejoin: round; stroke-linecap: round; }"
        ),
        # Not `currentColor`: inside a <pattern> its resolution varies between
        # renderers (it produced nothing under QuickLook's), and the hatch marks
        # a failed check — a signal that must not silently disappear. The
        # dark-mode block below overrides it explicitly instead.
        f".hatch {{ stroke: {INK_LIGHT}; stroke-width: 1.2; opacity: 0.75; }}",
    ]
    if highlight is not None:
        rules += [
            # Weight and z-order carry the distinction on their own, so the
            # graph still reads when printed in greyscale or seen by someone who
            # cannot separate the blue from the grey by hue.
            f".hist {{ stroke-width: {HISTORY_W}; }}",
            f".current {{ stroke-width: {CURRENT_W}; }}",
            ".legend-current { font-weight: 700; }",
        ]
    for i in range(series_count):
        light, _dark = colours[i]
        rules.append(f".s{i} {{ stroke: {light}; }}")
        rules.append(f".f{i} {{ fill: {light}; }}")
        # Past the end of the palette the colours repeat, and two versions
        # drawn in the same colour make the chart actively misleading — a
        # reader attributes one version's curve to another. Each wrap adds a
        # distinct dash pattern so colour+pattern stays unique. Legend swatches
        # carry the same pattern (see the legend renderer) so the key still
        # identifies a line. Applies to strokes only; bars are keyed by
        # position within their group, so a repeated fill is unambiguous there.
        #
        # Highlighted rendering does not wrap — the ramp is continuous and the
        # subject line must stay solid — so the dashes are palette-mode only.
        if highlight is None:
            dash = DASH_CYCLE[(i // len(PALETTE)) % len(DASH_CYCLE)]
            if dash:
                rules.append(f".s{i} {{ stroke-dasharray: {dash}; }}")

    dark_rules = [
        f":root {{ color: {INK_DARK}; }}",
        f".hatch {{ stroke: {INK_DARK}; }}",
    ]
    for i in range(series_count):
        light, dark = colours[i]
        if dark != light:
            dark_rules.append(f".s{i} {{ stroke: {dark}; }}")
            dark_rules.append(f".f{i} {{ fill: {dark}; }}")

    body = "\n    ".join(rules)
    dark_body = "\n      ".join(dark_rules)
    return (
        "  <style>\n"
        f"    {body}\n"
        "    @media (prefers-color-scheme: dark) {\n"
        f"      {dark_body}\n"
        "    }\n"
        "  </style>"
    )


def _svg_open(
    width: float,
    height: float,
    title: str,
    series_count: int,
    highlight: int | None,
) -> list[str]:
    """Document preamble: sizing, accessible title, stylesheet.

    No background rect is drawn — the page shows through, which is what lets one
    file sit on both a light and a dark GitHub page.
    """
    return [
        '<?xml version="1.0" encoding="UTF-8"?>',
        (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{fnum(width)}"'
            f' height="{fnum(height)}" viewBox="0 0 {fnum(width)} {fnum(height)}"'
            f' role="img" aria-labelledby="chart-title">'
        ),
        f'  <title id="chart-title">{esc(title)}</title>',
        _stylesheet(series_count, highlight),
    ]


HATCH_SPACING = 7.0


def _hatch_rect(x: float, y: float, w: float, h: float) -> list[str]:
    """Fill a rectangle with 45-degree stripes drawn as explicit clipped lines.

    An SVG `<pattern>` would be the obvious way to do this and is a fraction of
    the markup, but pattern support is the flakiest corner of the SVG renderers
    this output has to survive (macOS QuickLook drew nothing at all). The hatch
    is how a reader tells a timeout budget from a measurement, so it is worth
    the extra elements to have it composed of primitives every renderer handles.

    Emitted with a `<clipPath>` whose id is derived from the geometry, so it is
    stable across runs — no counters, no uuids.
    """
    if w <= 0 or h <= 0:
        return []
    # Geometry-derived id: same bar in the same place always gets the same id,
    # which keeps the bytes stable without threading a counter through.
    cid = f"hx{fnum(x).replace('.', '_').replace('-', 'n')}y{fnum(y).replace('.', '_').replace('-', 'n')}"
    out = [
        "  <defs>",
        f'    <clipPath id="{cid}">',
        f'      <rect x="{fnum(x)}" y="{fnum(y)}" width="{fnum(w)}" height="{fnum(h)}"/>',
        "    </clipPath>",
        "  </defs>",
        f'  <g clip-path="url(#{cid})">',
    ]
    # Sweep the diagonal's x-intercept from far enough left that the first
    # stripe still clips into the top-right corner, to the right edge.
    steps = int((w + h) / HATCH_SPACING) + 1
    for i in range(steps + 1):
        offset = -h + i * HATCH_SPACING
        x1 = x + offset
        y1 = y + h
        x2 = x1 + h
        y2 = y
        out.append(
            f'    <line class="hatch" x1="{fnum(x1)}" y1="{fnum(y1)}"'
            f' x2="{fnum(x2)}" y2="{fnum(y2)}"/>'
        )
    out.append("  </g>")
    return out


def _legend(
    items: Sequence[str],
    x: float,
    y: float,
    width: float,
    *,
    lines: bool = False,
    highlight: int | None = None,
) -> list[str]:
    """Swatch-and-label legend, laid out left to right in fixed-width columns.

    `lines=True` draws each swatch as a stroked line segment rather than a
    filled square, so that a series distinguished from a same-coloured one by
    its dash pattern (see `DASH_CYCLE`) is identifiable from the key. A filled
    square cannot show a dash pattern, which would leave two versions with
    identical swatches on any sweep of more than eight.

    The highlighted entry is named as well as coloured: it carries
    `CURRENT_SUFFIX` in bold, so a printed or screenshotted graph still says
    which release it is about.
    """
    out: list[str] = []
    col_w = width / LEGEND_COLS
    for i, label in enumerate(items):
        current = i == highlight
        col = i % LEGEND_COLS
        row = i // LEGEND_COLS
        lx = x + col * col_w
        ly = y + row * LEGEND_ROW_H
        if lines:
            weight = " current" if current else " hist" if highlight is not None else ""
            out.append(
                f'  <line class="series s{i}{weight}" x1="{fnum(lx)}" y1="{fnum(ly - 2)}" '
                f'x2="{fnum(lx + 14)}" y2="{fnum(ly - 2)}"/>'
            )
        else:
            out.append(
                f'  <rect class="f{i}" x="{fnum(lx)}" y="{fnum(ly - 8)}" width="12" height="12" rx="2"/>'
            )
        text = f"{label}{CURRENT_SUFFIX}" if current else label
        cls = ' class="legend-current"' if current else ""
        out.append(
            f'  <text{cls} x="{fnum(lx + 18)}" y="{fnum(ly + 2)}">{esc(text)}</text>'
        )
    return out


def legend_height(count: int) -> float:
    rows = max(1, math.ceil(count / LEGEND_COLS)) if count else 0
    return rows * LEGEND_ROW_H


def _axes(
    x0: float,
    y0: float,
    x1: float,
    y1: float,
    x_ticks: Sequence[tuple[float, str]],
    y_ticks: Sequence[tuple[float, str]],
    x_title: str,
    y_title: str,
    rotate_x_labels: bool = False,
    x_title_offset: float = AXIS_LABEL_H - 4,
) -> list[str]:
    """Draw the axis lines, tick marks, tick labels, gridlines and axis titles.

    `x_ticks`/`y_ticks` arrive as (pixel, label) so the caller owns the scale and
    this function stays scale-agnostic (it is shared by the linear line charts
    and the log-axis bar chart). `x_title_offset` exists because rotated group
    labels need far more clearance below the axis than horizontal ones.
    """
    out: list[str] = []
    for px, label in y_ticks:
        out.append(
            f'  <line class="grid" x1="{fnum(x0)}" y1="{fnum(px)}" x2="{fnum(x1)}" y2="{fnum(px)}"/>'
        )
        out.append(
            f'  <line class="chrome" x1="{fnum(x0 - 4)}" y1="{fnum(px)}" x2="{fnum(x0)}" y2="{fnum(px)}"/>'
        )
        out.append(
            f'  <text x="{fnum(x0 - 8)}" y="{fnum(px + 3.5)}" text-anchor="end">{esc(label)}</text>'
        )
    for px, label in x_ticks:
        out.append(
            f'  <line class="chrome" x1="{fnum(px)}" y1="{fnum(y1)}" x2="{fnum(px)}" y2="{fnum(y1 + 4)}"/>'
        )
        if rotate_x_labels:
            # Long check ids overlap badly when horizontal; -40 degrees is the
            # shallowest angle that keeps ~20 characters legible side by side.
            out.append(
                f'  <text x="{fnum(px)}" y="{fnum(y1 + 14)}" text-anchor="end"'
                f' transform="rotate(-40 {fnum(px)} {fnum(y1 + 14)})">{esc(label)}</text>'
            )
        else:
            out.append(
                f'  <text x="{fnum(px)}" y="{fnum(y1 + 16)}" text-anchor="middle">{esc(label)}</text>'
            )

    out.append(
        f'  <line class="chrome" x1="{fnum(x0)}" y1="{fnum(y0)}" x2="{fnum(x0)}" y2="{fnum(y1)}"/>'
    )
    out.append(
        f'  <line class="chrome" x1="{fnum(x0)}" y1="{fnum(y1)}" x2="{fnum(x1)}" y2="{fnum(y1)}"/>'
    )
    out.append(
        f'  <text class="axis-title" x="{fnum((x0 + x1) / 2)}" y="{fnum(y1 + x_title_offset)}"'
        f' text-anchor="middle">{esc(x_title)}</text>'
    )
    out.append(
        f'  <text class="axis-title" x="{fnum(x0 - 56)}" y="{fnum((y0 + y1) / 2)}"'
        f' text-anchor="middle" transform="rotate(-90 {fnum(x0 - 56)} {fnum((y0 + y1) / 2)})">'
        f"{esc(y_title)}</text>"
    )
    return out


def _empty_note(x0: float, y0: float, x1: float, y1: float, message: str) -> list[str]:
    """Placeholder for a chart with nothing to draw.

    Emitting a valid, labelled SVG beats emitting nothing: a missing file breaks
    the image reference in the release notes, whereas an empty chart says plainly
    that the data was absent.
    """
    return [
        (
            f'  <text class="note" x="{fnum((x0 + x1) / 2)}" y="{fnum((y0 + y1) / 2)}"'
            f' text-anchor="middle">{esc(message)}</text>'
        )
    ]


# --------------------------------------------------------------------------
# Charts
# --------------------------------------------------------------------------


def line_chart(
    title: str,
    x_title: str,
    y_title: str,
    series: Sequence[tuple[str, Sequence[tuple[float, float]]]],
    footnote: str = "",
    highlight: int | None = None,
) -> str:
    """Render a multi-series line chart, one line per (label, points) pair.

    Single-point series are drawn as a dot: a `polyline` with one vertex is
    invisible, and a version whose run produced exactly one sample should still
    show up rather than silently disappearing from the legend's story.

    `highlight` is an index into `series`. That line is drawn thick, in
    `HIGHLIGHT`, and — critically — *last*, so eighteen releases' worth of
    context can never paint over the one the reader came for.
    """
    labels = [name for name, _ in series]
    legend_h = legend_height(len(labels))
    height = (
        MARGIN_TOP
        + PLOT_H
        + AXIS_LABEL_H
        + legend_h
        + (FOOTNOTE_H if footnote else 0)
        + 12
    )
    width = MARGIN_LEFT + PLOT_W + MARGIN_RIGHT

    x0, y0 = float(MARGIN_LEFT), float(MARGIN_TOP)
    x1, y1 = x0 + PLOT_W, y0 + PLOT_H

    out = _svg_open(width, height, title, len(labels), highlight)
    out.append(
        f'  <text class="title" x="{fnum(x0)}" y="{fnum(MARGIN_TOP - 26)}">{esc(title)}</text>'
    )

    all_points = [p for _, pts in series for p in pts]
    if not all_points:
        out += _axes(x0, y0, x1, y1, [], [], x_title, y_title)
        out += _empty_note(x0, y0, x1, y1, "no timeline samples")
        out.append("</svg>")
        return "\n".join(out) + "\n"

    xs = [p[0] for p in all_points]
    ys = [p[1] for p in all_points]
    x_lo, x_hi, x_tick_vals = nice_ticks(min(xs), max(xs))
    # Anchor the value axis at zero when the data is non-negative: an RSS chart
    # whose baseline floats makes a 5% wobble look like a cliff.
    y_lo, y_hi, y_tick_vals = nice_ticks(min(min(ys), 0.0), max(ys))

    sx = make_scale(x_lo, x_hi, x0, x1, log=False)
    sy = make_scale(y_lo, y_hi, y1, y0, log=False)

    out += _axes(
        x0,
        y0,
        x1,
        y1,
        [(sx(v), ftick(v)) for v in x_tick_vals],
        [(sy(v), ftick(v)) for v in y_tick_vals],
        x_title,
        y_title,
    )

    # Highlighted line last: with eighteen releases on one set of axes, a
    # bright-blue line painted in version order is repeatedly crossed and
    # partially hidden by the greys drawn after it.
    order = [i for i in range(len(series)) if i != highlight]
    if highlight is not None:
        order.append(highlight)
    for i in order:
        _name, points = series[i]
        if not points:
            continue
        weight = ""
        if highlight is not None:
            weight = " current" if i == highlight else " hist"
        if len(points) == 1:
            px, py = sx(points[0][0]), sy(points[0][1])
            radius = 5 if i == highlight else 3
            out.append(
                f'  <circle class="f{i}" cx="{fnum(px)}" cy="{fnum(py)}" r="{radius}"/>'
            )
            continue
        coords = " ".join(f"{fnum(sx(x))},{fnum(sy(y))}" for x, y in points)
        out.append(f'  <polyline class="series s{i}{weight}" points="{coords}"/>')

    legend_y = y1 + AXIS_LABEL_H + 12
    # Line swatches: past eight series the palette wraps and the dash pattern
    # is what tells two same-coloured lines apart, so the key has to show it.
    out += _legend(labels, x0, legend_y, PLOT_W, lines=True, highlight=highlight)
    if footnote:
        out.append(
            f'  <text class="note" x="{fnum(x0)}" y="{fnum(legend_y + legend_h + 10)}">{esc(footnote)}</text>'
        )
    out.append("</svg>")
    return "\n".join(out) + "\n"


@dataclass(frozen=True)
class Bar:
    """One bar: a value plus whether the underlying check actually succeeded."""

    value: float
    ok: bool


def grouped_bar_chart(
    title: str,
    x_title: str,
    y_title: str,
    group_labels: Sequence[str],
    series: Sequence[tuple[str, Sequence[Bar | None]]],
    log: bool,
    footnote: str = "",
    highlight: int | None = None,
) -> str:
    """Render grouped bars: one group per check, one bar per version.

    `series` values are positional against `group_labels`; `None` means that
    version has no measurement for that check (it was added or removed between
    releases), and leaves a gap rather than a zero-height bar that would read as
    "instant".

    Failed checks are drawn hatched with a dashed outline. Their `wall_s` is a
    timeout budget, so showing them as an ordinary bar would assert a duration
    that was never observed.

    `highlight` colours one version's bar in every group `HIGHLIGHT` blue while
    the rest ride the ageing ramp. Bars occupy disjoint slots, so this is purely
    about colour — but with nineteen 2px bars per group it is the whole reason
    the reader can find the release in the picture at all.
    """
    labels = [name for name, _ in series]
    legend_h = legend_height(len(labels))
    bar_label_h = AXIS_LABEL_H + 46  # extra room for the rotated group labels
    height = MARGIN_TOP + PLOT_H + bar_label_h + legend_h + FOOTNOTE_H + 12
    width = MARGIN_LEFT + PLOT_W + MARGIN_RIGHT

    x0, y0 = float(MARGIN_LEFT), float(MARGIN_TOP)
    x1, y1 = x0 + PLOT_W, y0 + PLOT_H

    out = _svg_open(width, height, title, len(labels), highlight)
    out.append(
        f'  <text class="title" x="{fnum(x0)}" y="{fnum(MARGIN_TOP - 26)}">{esc(title)}</text>'
    )

    values = [b.value for _, bars in series for b in bars if b is not None]
    if not group_labels or not values:
        out += _axes(x0, y0, x1, y1, [], [], x_title, y_title)
        out += _empty_note(x0, y0, x1, y1, "no checks recorded")
        out.append("</svg>")
        return "\n".join(out) + "\n"

    if log:
        positive = [v for v in values if v > 0]
        # Drop the requested floor below the smallest value before snapping to a
        # decade. Without this, a fastest check of exactly 0.01s lands on the
        # axis floor and draws a zero-height bar — indistinguishable from "no
        # data" for precisely the measurements the log axis exists to reveal.
        lo = (min(positive) / 2.0) if positive else 0.01
        y_lo, y_hi, y_tick_vals = log_ticks(lo, max(values))
    else:
        y_lo, y_hi, y_tick_vals = nice_ticks(0.0, max(values))

    sy = make_scale(y_lo, y_hi, y1, y0, log=log)
    baseline = y1

    group_w = PLOT_W / len(group_labels)
    # Leave 20% of each group as a gutter so adjacent groups stay readable.
    bar_w = (group_w * 0.8) / max(len(series), 1)

    # Same reasoning as the line chart: the highlighted version is emitted last
    # within each group. Bars do not overlap, so this only matters for the
    # dashed failure outline, which is 1.5px wide and can bleed into the
    # neighbouring 2px slot.
    bar_order = [i for i in range(len(series)) if i != highlight]
    if highlight is not None:
        bar_order.append(highlight)

    x_ticks: list[tuple[float, str]] = []
    for gi, label in enumerate(group_labels):
        centre = x0 + group_w * (gi + 0.5)
        x_ticks.append((centre, label))
        for si in bar_order:
            _name, bars = series[si]
            bar = bars[gi] if gi < len(bars) else None
            if bar is None:
                continue
            bx = centre - (len(series) * bar_w) / 2 + si * bar_w
            by = sy(bar.value)
            bh = max(baseline - by, 0.0)
            if bar.ok:
                out.append(
                    f'  <rect class="f{si}" x="{fnum(bx)}" y="{fnum(by)}"'
                    f' width="{fnum(bar_w * 0.9)}" height="{fnum(bh)}"/>'
                )
            else:
                out.append(
                    f'  <rect class="f{si}" x="{fnum(bx)}" y="{fnum(by)}"'
                    f' width="{fnum(bar_w * 0.9)}" height="{fnum(bh)}" opacity="0.25"/>'
                )
                out += _hatch_rect(bx, by, bar_w * 0.9, bh)
                out.append(
                    f'  <rect class="s{si}" fill="none" stroke-width="1.5" stroke-dasharray="3 2"'
                    f' x="{fnum(bx)}" y="{fnum(by)}"'
                    f' width="{fnum(bar_w * 0.9)}" height="{fnum(bh)}"/>'
                )

    out += _axes(
        x0,
        y0,
        x1,
        y1,
        x_ticks,
        [(sy(v), ftick(v)) for v in y_tick_vals],
        x_title,
        y_title,
        rotate_x_labels=True,
        x_title_offset=bar_label_h - 6,
    )

    legend_y = y1 + bar_label_h + 8
    out += _legend(labels, x0, legend_y, PLOT_W, highlight=highlight)
    note = "hatched, dashed bars are checks that failed or timed out (value is the timeout budget)"
    if footnote:
        note = f"{note} — {footnote}"
    out.append(
        f'  <text class="note" x="{fnum(x0)}" y="{fnum(legend_y + legend_h + 10)}">{esc(note)}</text>'
    )
    out.append("</svg>")
    return "\n".join(out) + "\n"


# --------------------------------------------------------------------------
# Chart assembly
# --------------------------------------------------------------------------


def _highlight_note(runs: Sequence[Run], highlight: int | None) -> str:
    """Footnote clause naming the highlighted release, or empty when there is none."""
    if highlight is None:
        return ""
    return f"{runs[highlight].version} is drawn in blue; earlier releases fade with age"


def build_memory_svg(runs: Sequence[Run], highlight: int | None = None) -> str:
    series = [(r.version, rss_series(r)) for r in runs]
    note = "a line that keeps climbing to the right never plateaued — that is the leak shape"
    if extra := _highlight_note(runs, highlight):
        note = f"{extra} — {note}"
    return line_chart(
        "Resident memory over the benchmark run",
        "elapsed (s)",
        "RSS (MiB)",
        series,
        footnote=note,
        highlight=highlight,
    )


def build_cpu_svg(runs: Sequence[Run], highlight: int | None = None) -> str:
    series = [(r.version, cpu_series(r)) for r in runs]
    note = (
        "derived from cumulative CPU seconds; above 100% means more than one core busy"
    )
    if extra := _highlight_note(runs, highlight):
        note = f"{extra} — {note}"
    return line_chart(
        "CPU utilisation over the benchmark run",
        "elapsed (s)",
        "CPU (%)",
        series,
        footnote=note,
        highlight=highlight,
    )


def collect_groups(runs: Sequence[Run]) -> list[str]:
    """Union of check ids across all runs, ordered by `ord` then id.

    Taking the union (rather than one run's list) means a check added in the
    newest release still gets a group, with gaps for the versions that predate
    it. The minimum `ord` wins when a check was renumbered, and the id breaks
    ties so the order cannot depend on which file was read first.
    """
    ords: dict[str, int] = {}
    for run in runs:
        for check in run.checks:
            if check.id not in ords or check.ord < ords[check.id]:
                ords[check.id] = check.ord
    return [cid for cid, _ in sorted(ords.items(), key=lambda kv: (kv[1], kv[0]))]


def build_walltime_svg(
    runs: Sequence[Run], log: bool, highlight: int | None = None
) -> str:
    groups = collect_groups(runs)
    series: list[tuple[str, list[Bar | None]]] = []
    for run in runs:
        by_id = {c.id: c for c in run.checks}
        bars: list[Bar | None] = []
        for cid in groups:
            check = by_id.get(cid)
            bars.append(Bar(check.wall_s, check.ok) if check else None)
        series.append((run.version, bars))
    axis = "wall time (s, log10)" if log else "wall time (s)"
    return grouped_bar_chart(
        "Per-check wall time by version",
        "check",
        axis,
        groups,
        series,
        log=log,
        footnote=_highlight_note(runs, highlight),
        highlight=highlight,
    )


# --------------------------------------------------------------------------
# Markdown fragment
# --------------------------------------------------------------------------


def _md_cell(value: float) -> str:
    return f"{value:.2f}"


def _pct_delta(current: float, previous: float) -> str:
    """Signed percentage change, or `n/a` when the baseline is zero."""
    if previous == 0.0:
        return "n/a"
    return f"{100.0 * (current - previous) / previous:+.1f}%"


def build_comparison_lines(runs: Sequence[Run], highlight: int) -> list[str]:
    """Head-to-head of the highlighted release against the one before it.

    This is the number a release note actually wants, and computing it here
    rather than by eye is what stops it being quietly wrong.

    Emitted only when both runs were measured on the same host. Half of the
    committed history was recorded on an Apple M1 Max and the rest on a
    four-core Linux runner; a "+180% CPU" derived across that boundary would be
    a statement about the hardware presented as a statement about the release.
    """
    if highlight <= 0:
        return []
    current, previous = runs[highlight], runs[highlight - 1]
    if (current.platform, current.host_cpu) != (previous.platform, previous.host_cpu):
        return [
            (
                f"_`{current.version}` and `{previous.version}` were measured on "
                f"different hosts ({previous.host_cpu} → {current.host_cpu}), so no "
                "release-over-release delta is quoted: the difference would be "
                "mostly hardware._"
            ),
            "",
        ]
    rows = [
        ("Peak RSS (MiB)", current.peak_rss_mib, previous.peak_rss_mib),
        ("RSS growth (MiB)", current.rss_growth_mib, previous.rss_growth_mib),
        ("Total CPU (s)", current.total_cpu_s, previous.total_cpu_s),
        ("Total wall (s)", current.total_wall_s, previous.total_wall_s),
    ]
    out = [
        (
            f"**`{current.version}` vs `{previous.version}`** (same host — "
            f"{current.host_cpu}, {current.host_cores} cores)"
        ),
        "",
        f"| Measure | {previous.version} | {current.version} | Change |",
        "| --- | ---: | ---: | ---: |",
    ]
    out += [
        f"| {name} | {_md_cell(prev)} | {_md_cell(cur)} | {_pct_delta(cur, prev)} |"
        for name, cur, prev in rows
    ]
    out.append("")
    return out


def build_summary_md(
    runs: Sequence[Run], log: bool, highlight: int | None = None
) -> str:
    """Assemble the release-notes fragment.

    Starts at `##` deliberately: this is pasted inside a release-notes document
    that already owns the single top-level heading, and a second `#` would break
    its outline.
    """
    lines: list[str] = ["## Performance", ""]
    if not runs:
        lines += ["_No benchmark results were found._", ""]
        return "\n".join(lines)

    if highlight is not None:
        lines += [
            (
                f"`{runs[highlight].version}` is this release: it is the blue line in "
                "every graph below, and the bold row in the table. Earlier releases are "
                "drawn in grey, fading with age."
            ),
            "",
        ]

    lines += [
        "| Version | Peak RSS (MiB) | Final RSS (MiB) | RSS growth (MiB) | Total CPU (s) | Total wall (s) | Failed checks |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for i, run in enumerate(runs):
        lines.append(
            "| {v} | {peak} | {final} | {growth} | {cpu} | {wall} | {failed} |".format(
                # Bold, not a marker character: the emphasis survives the table
                # being pasted anywhere GitHub-flavoured Markdown renders.
                v=f"**{run.version}**" if i == highlight else run.version,
                peak=_md_cell(run.peak_rss_mib),
                final=_md_cell(run.final_rss_mib),
                growth=_md_cell(run.rss_growth_mib),
                cpu=_md_cell(run.total_cpu_s),
                wall=_md_cell(run.total_wall_s),
                failed=run.failed_checks,
            )
        )
    lines.append("")

    if highlight is not None:
        lines += build_comparison_lines(runs, highlight)

    # Name the failures explicitly. A "1" in the table above is easy to skim
    # past; the check that timed out is the thing a reader needs.
    failures: list[str] = []
    for run in runs:
        for check in run.checks:
            if not check.ok:
                suffix = f" — {check.note}" if check.note else ""
                failures.append(
                    f"- `{run.version}` **{check.id}** ({check.label}) failed or timed out "
                    f"at {_md_cell(check.wall_s)}s{suffix}"
                )
    if failures:
        lines += ["**Failed checks**", "", *failures, ""]

    lines += [
        "![Resident memory over the benchmark run](memory.svg)",
        "",
        "![CPU utilisation over the benchmark run](cpu.svg)",
        "",
        "![Per-check wall time by version](walltime.svg)",
        "",
    ]

    newest = runs[-1]
    axis = "log10" if log else "linear"
    # Optional provenance: only mentioned when bench.py recorded it, so the
    # sentence stays clean for a hand-written or older results file.
    corpus = f"{newest.corpus_files}-file corpus"
    if newest.scope:
        corpus += f", scope `{newest.scope}`"
    if newest.corpus_revision:
        corpus += f", revision `{newest.corpus_revision}`"
    lines += [
        (
            f"_Generated by `scripts/perf/report.py` from {len(runs)} result "
            f"file(s) on {newest.host_cpu} ({newest.host_cores} cores, "
            f"{newest.host_mem_gb} GiB, {newest.platform}) over a "
            f"{corpus}. Wall-time axis: {axis}._"
        ),
        "",
    ]
    return "\n".join(lines)


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------


def write_text(path: Path, content: str) -> Path:
    """Write UTF-8 with LF endings regardless of platform.

    `newline=""` stops Windows from translating to CRLF, which would make the
    committed bytes differ by the machine that generated them.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as fh:
        fh.write(content)
    return path


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="report.py",
        description="Render benchmark results into deterministic SVG graphs and a Markdown fragment.",
    )
    parser.add_argument(
        "--results",
        type=Path,
        default=HERE / "results",
        help="directory of results/*.json files, as written by bench.py (default: %(default)s)",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=HERE / "graphs",
        help="output directory for the SVGs and summary.md (default: %(default)s)",
    )
    parser.add_argument(
        "--log",
        action=argparse.BooleanOptionalAction,
        default=True,
        help=(
            "log10 Y axis on the wall-time graph (default: enabled — check times span"
            " orders of magnitude and a linear axis hides the fast ones)"
        ),
    )
    parser.add_argument(
        "--only",
        default=None,
        help="comma-separated list of versions to include (default: all)",
    )
    parser.add_argument(
        "--highlight",
        default=None,
        metavar="VERSION",
        help=(
            "version to draw as the current release — bright blue, thick, on top,"
            " with every other version on a neutral ageing ramp"
            " (default: the newest result present)"
        ),
    )
    parser.add_argument(
        "--no-highlight",
        action="store_true",
        help=(
            "give every version an equal categorical colour instead, for comparing"
            " arbitrary versions where none of them is the subject"
        ),
    )
    return parser.parse_args(argv)


def resolve_highlight(runs: Sequence[Run], args: argparse.Namespace) -> int | None:
    """Index of the series to draw as the current release, or None.

    An explicit `--highlight` that matches nothing is a hard error rather than a
    silent fallback to the newest: the usual way to reach it is a typo or a
    benchmark that never got run, and both produce a graph that says "here is
    where 2.1.19 sits" while showing 2.1.18 in blue.
    """
    if args.no_highlight or not runs:
        return None
    if not args.highlight:
        return len(runs) - 1
    wanted = args.highlight.lstrip("v")
    for i, run in enumerate(runs):
        if run.version == wanted:
            return i
    available = ", ".join(r.version for r in runs)
    raise ValueError(
        f"--highlight {args.highlight!r} matches no loaded result "
        f"(have: {available or 'none'})"
    )


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)

    if not args.results.is_dir():
        print(f"error: results directory not found: {args.results}", file=sys.stderr)
        return 2

    only = [v.strip() for v in args.only.split(",") if v.strip()] if args.only else None
    try:
        runs = load_runs(args.results, only)
    except (ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if not runs:
        print(f"warning: no results matched in {args.results}", file=sys.stderr)

    try:
        highlight = resolve_highlight(runs, args)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    for i, run in enumerate(runs):
        marker = " <- highlighted as the current release" if i == highlight else ""
        print(
            f"{run.version}: peak {run.peak_rss_mib:.2f} MiB, final {run.final_rss_mib:.2f} MiB "
            f"(growth {run.rss_growth_mib:+.2f} MiB), cpu {run.total_cpu_s:.2f}s, "
            f"wall {run.total_wall_s:.2f}s, {len(run.checks)} checks, "
            f"{run.failed_checks} failed, {len(run.timeline)} samples{marker}"
        )

    written = [
        write_text(args.out / "memory.svg", build_memory_svg(runs, highlight)),
        write_text(args.out / "cpu.svg", build_cpu_svg(runs, highlight)),
        write_text(
            args.out / "walltime.svg", build_walltime_svg(runs, args.log, highlight)
        ),
        write_text(
            args.out / "summary.md", build_summary_md(runs, args.log, highlight)
        ),
    ]
    for path in written:
        print(f"wrote {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
