"""Per-stem categorisation lookup for the Tcl 9 tcltest harness.

See ``tests/baselines/tcl9-tcltest/categories/README.md`` for the
bucket definitions and file format.  This module's job is the small
plumbing layer: read the TOML for a stem, expose a ``bucket_of`` query
that returns ``"must_pass"`` for unlisted test IDs (the deliberately
fail-closed default), and aggregate per-stem counts so the harness
can emit them in its JSON record without each call site re-parsing
the file.

Synthesised from three parallel categorisation efforts:

* the ``tcl9-arith-wording`` stream's per-stem split with cluster-level
  hand-off notes for un-binned residuals,
* the ``tcl9-frames-aliases-trace`` stream's ``trap_allowed`` + cached
  loader + ``tcltest::configure -skip`` injection,
* the ``tcl9-codegen-misc`` stream's ``[baseline]`` cap, overlap and
  unknown-key validation.

The combined surface keeps every stream's gating semantics so the
three feature branches can merge into ``main`` without conflict.
"""

from __future__ import annotations

import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from threading import Lock

# Repo-relative path to the per-stem TOML files.  Resolved once at
# import time so a ``cd`` mid-test doesn't break the lookup.
_REPO_ROOT = Path(__file__).resolve().parents[2]
_CATEGORIES_DIR = _REPO_ROOT / "tests" / "baselines" / "tcl9-tcltest" / "categories"


# Recognised bucket names — the keys we accept at the TOML top level.
# Test IDs not appearing in any of these implicitly fall into
# :data:`DEFAULT_BUCKET`.  ``must_pass`` is the "everything else"
# bucket; listing it explicitly in a TOML is allowed but redundant.
#
# ``skip`` is the bucket for tests we explicitly do not run — they
# probe C-Tcl bytecode shape, instruction count, ``info frame``
# line numbers tied to the bcc, the ``::tcl::test`` /
# ``::tcl::dict::*`` internal namespaces, or other implementation
# details our WASM runtime cannot meaningfully emulate.  The harness
# passes the list to ``tcltest::configure -skip`` so they show up in
# the skipped count, never as failures, and cost zero runtime.
BUCKETS = ("must_pass", "good_to_have", "just_to_match_ctcl", "skip")
DEFAULT_BUCKET = "must_pass"

# Top-level TOML keys we recognise.  Unknown keys are rejected so a
# typo (``goodto_have = [...]``) doesn't silently disable a list.
_KNOWN_KEYS = frozenset(BUCKETS) | {"baseline", "trap_allowed"}

# Baseline-table keys we recognise.
_BASELINE_KEYS = frozenset(
    {
        "good_to_have_failing",
        "just_to_match_ctcl_failing",
        "skip_failing",
    }
)


@dataclass(frozen=True)
class StemCategories:
    """Loaded categorisation for a single tcltest stem.

    ``test_to_bucket`` maps an ID *suffix* (e.g. ``"3.5"`` for
    ``dict-3.5``) to its bucket name.  ``trap_allowed`` is the
    stem-wide hatch for stems that currently trap mid-run before
    reaching a tcltest summary — when true the harness records the
    trap as a tracked failure rather than promoting it to a hard
    fail.

    The ``*_baseline`` fields are the recorded snapshot counts at
    triage time.  ``good_to_have_baseline`` is the cap the gate
    enforces (current count must not exceed it — drops are welcome,
    growth fires the gate so a previously-passing GOOD_TO_HAVE test
    that starts failing surfaces even though the test name was
    already listed in the bucket).  The ``just_to_match_ctcl`` and
    ``skip`` baselines are recorded for visibility only — neither
    direction is gating.

    ``raw`` retains the original parsed TOML for the JSON record so
    the report consumer can see exactly what was loaded.
    """

    stem: str
    test_to_bucket: dict[str, str] = field(default_factory=dict)
    trap_allowed: bool = False
    good_to_have_baseline: int = 0
    just_to_match_ctcl_baseline: int = 0
    skip_baseline: int = 0
    raw: dict = field(default_factory=dict)

    def bucket_of(self, test_id_or_suffix: str) -> str:
        """Return the bucket for *test_id_or_suffix*.

        Accepts either ``"dict-3.5"`` or the bare ``"3.5"`` suffix —
        callers parsing tcltest output have full IDs while seed
        authors edit suffixes, so the function tolerates both.
        Unlisted IDs return :data:`DEFAULT_BUCKET` (``must_pass``).
        """
        if not test_id_or_suffix:
            return DEFAULT_BUCKET
        suffix = _strip_stem(self.stem, test_id_or_suffix)
        return self.test_to_bucket.get(suffix, DEFAULT_BUCKET)

    def skip_full_ids(self) -> list[str]:
        """Return the list of full ``<stem>-<suffix>`` IDs in the
        ``skip`` bucket — the form ``tcltest::configure -skip``
        wants.  Order is the seed-file order so the generated
        directive is reproducible across runs.
        """
        return [
            f"{self.stem}-{suffix}"
            for suffix, bucket in self.test_to_bucket.items()
            if bucket == "skip"
        ]


def _strip_stem(stem: str, test_id: str) -> str:
    """Return *test_id* with its ``<stem>-`` prefix removed if present.

    ``upvar-3.1`` → ``"3.1"``.  Bare suffixes pass through unchanged
    so seed-file editing and runtime lookup share one code path.
    """
    prefix = f"{stem}-"
    if test_id.startswith(prefix):
        return test_id[len(prefix) :]
    return test_id


# Per-stem cache: ``load_categories`` is called from every test_runs
# invocation in the suite, so re-parsing the TOML on each call would
# be needlessly expensive (a 200-test sweep does it 200 times).  The
# Lock is a defensive measure against pytest-xdist worker forks; in
# practice the cache is populated once per worker.
_cache: dict[str, StemCategories] = {}
_cache_lock = Lock()


def load_categories(stem: str) -> StemCategories:
    """Load the categorisation for *stem*, caching the result.

    Returns a :class:`StemCategories` with empty mappings (so every
    test defaults to ``must_pass``) when the file is absent — that is
    the new-stem onboarding path.  Malformed TOML, duplicate IDs,
    overlap, unknown keys, or a baseline that exceeds the listed-test
    count all raise :class:`RuntimeError` so the seed-file mistake
    surfaces at the next ``test_runs`` invocation rather than silently
    mis-categorising failures.
    """
    with _cache_lock:
        cached = _cache.get(stem)
        if cached is not None:
            return cached
        loaded = _load_uncached(stem)
        _cache[stem] = loaded
        return loaded


def _load_uncached(stem: str) -> StemCategories:
    path = _CATEGORIES_DIR / f"{stem}.toml"
    if not path.exists():
        return StemCategories(stem=stem)
    raw = tomllib.loads(path.read_text(encoding="utf-8"))

    # Reject unknown top-level keys so a typo doesn't silently
    # disable a category list (e.g. ``goodto_have = [...]``).
    unknown_top = set(raw.keys()) - _KNOWN_KEYS
    if unknown_top:
        msg = (
            f"{path}: unknown top-level keys: {sorted(unknown_top)} "
            f"(allowed: {sorted(_KNOWN_KEYS)})"
        )
        raise RuntimeError(msg)

    test_to_bucket: dict[str, str] = {}
    bucket_counts: dict[str, int] = {b: 0 for b in BUCKETS}
    for bucket in BUCKETS:
        ids = raw.get(bucket, [])
        if not isinstance(ids, list):
            msg = f"{path}: top-level '{bucket}' must be an array of strings"
            raise RuntimeError(msg)
        for tid in ids:
            if not isinstance(tid, str):
                msg = f"{path}: '{bucket}' entry {tid!r} is not a string"
                raise RuntimeError(msg)
            if tid in test_to_bucket and test_to_bucket[tid] != bucket:
                # Catching this keeps the seed file unambiguous: every
                # ID lives in exactly one bucket, with the rest
                # implicitly must_pass.
                msg = (
                    f"{path}: test id {tid!r} listed in both "
                    f"{test_to_bucket[tid]!r} and {bucket!r}"
                )
                raise RuntimeError(msg)
            if tid not in test_to_bucket:
                bucket_counts[bucket] += 1
            test_to_bucket[tid] = bucket

    trap_allowed = bool(raw.get("trap_allowed", False))

    # Parse the optional ``[baseline]`` table.  A missing table or
    # missing key is fine — we default each cap to the listed count
    # so a fresh manifest (still being built up) doesn't fire the
    # cap accidentally.  Reject unknown keys here too.
    baseline = raw.get("baseline", {}) or {}
    if not isinstance(baseline, dict):
        msg = f"{path}: [baseline] must be a table"
        raise RuntimeError(msg)
    unknown_baseline = set(baseline.keys()) - _BASELINE_KEYS
    if unknown_baseline:
        msg = (
            f"{path}: unknown [baseline] keys: {sorted(unknown_baseline)} "
            f"(allowed: {sorted(_BASELINE_KEYS)})"
        )
        raise RuntimeError(msg)

    good_baseline = int(baseline.get("good_to_have_failing", bucket_counts["good_to_have"]))
    ctcl_baseline = int(
        baseline.get("just_to_match_ctcl_failing", bucket_counts["just_to_match_ctcl"])
    )
    skip_baseline = int(baseline.get("skip_failing", bucket_counts["skip"]))

    # Sanity: a baseline that exceeds the listed-test count is a
    # bookkeeping error — every failing GOOD_TO_HAVE test must be
    # listed by name so we can detect "this specific test is no
    # longer failing in the way we expected" or "a previously-
    # passing one regressed".
    for label, listed_count, recorded in (
        ("good_to_have", bucket_counts["good_to_have"], good_baseline),
        ("just_to_match_ctcl", bucket_counts["just_to_match_ctcl"], ctcl_baseline),
        ("skip", bucket_counts["skip"], skip_baseline),
    ):
        if recorded > listed_count:
            msg = (
                f"{path}: baseline.{label}_failing ({recorded}) exceeds "
                f"the number of listed {label} tests ({listed_count})"
            )
            raise RuntimeError(msg)

    return StemCategories(
        stem=stem,
        test_to_bucket=test_to_bucket,
        trap_allowed=trap_allowed,
        good_to_have_baseline=good_baseline,
        just_to_match_ctcl_baseline=ctcl_baseline,
        skip_baseline=skip_baseline,
        raw=raw,
    )


@dataclass
class BucketReport:
    """Per-bucket counts and the IDs in each, for a single test run.

    The harness builds one of these from the failure list parsed out
    of the tcltest summary, then surfaces the counts in the JSON
    triage record and uses ``must_pass`` plus the ``good_to_have``
    cap as the gate condition.
    """

    must_pass_failures: list[str] = field(default_factory=list)
    good_to_have_failures: list[str] = field(default_factory=list)
    just_to_match_ctcl_failures: list[str] = field(default_factory=list)
    skip_failures: list[str] = field(default_factory=list)

    @property
    def total(self) -> int:
        return (
            len(self.must_pass_failures)
            + len(self.good_to_have_failures)
            + len(self.just_to_match_ctcl_failures)
            + len(self.skip_failures)
        )

    def to_dict(self) -> dict:
        return {
            "must_pass": self.must_pass_failures,
            "good_to_have": self.good_to_have_failures,
            "just_to_match_ctcl": self.just_to_match_ctcl_failures,
            "skip": self.skip_failures,
        }


def bucket_failures(stem: str, failed_ids: list[str]) -> BucketReport:
    """Partition *failed_ids* into a :class:`BucketReport` for *stem*.

    The IDs are full tcltest names (``"dict-3.5"``); the loader's
    suffix-stripping handles the prefix.  Order is preserved within
    each bucket so the JSON record is reproducible.

    Failed IDs that fall in the ``skip`` bucket land in
    ``skip_failures`` rather than blocking the gate — but emitting
    one is still suspicious, because the harness should have asked
    tcltest to skip those tests via ``tcltest::configure -skip``.
    Surface in the JSON record so a missed injection shows up in
    review.
    """
    cats = load_categories(stem)
    report = BucketReport()
    for fid in failed_ids:
        bucket = cats.bucket_of(fid)
        if bucket == "must_pass":
            report.must_pass_failures.append(fid)
        elif bucket == "good_to_have":
            report.good_to_have_failures.append(fid)
        elif bucket == "just_to_match_ctcl":
            report.just_to_match_ctcl_failures.append(fid)
        else:
            report.skip_failures.append(fid)
    return report


@dataclass(frozen=True)
class GateOutcome:
    """Result of :func:`gate`: pass / fail and a one-line reason.

    ``passed`` is the AND of:

    1. ``len(report.must_pass_failures) == 0``
    2. ``len(report.good_to_have_failures) <= cats.good_to_have_baseline``

    Drops in any bucket are always welcome — only growth fires.
    """

    passed: bool
    reason: str


def gate(cats: StemCategories, report: BucketReport) -> GateOutcome:
    """Decide pass / fail for one stem given its bucket report.

    ``must_pass`` failures gate immediately.  ``good_to_have`` growth
    past the recorded baseline also gates — that catches the case
    where a previously-passing GOOD_TO_HAVE test regresses, even
    though the test name is already listed in the bucket (a list-
    membership-only gate would silently accept the regression).
    """
    if report.must_pass_failures:
        sample = ", ".join(report.must_pass_failures[:5])
        suffix = " …" if len(report.must_pass_failures) > 5 else ""
        return GateOutcome(
            passed=False,
            reason=(
                f"{len(report.must_pass_failures)} MUST_PASS test(s) failed: "
                f"{sample}{suffix}"
            ),
        )
    if len(report.good_to_have_failures) > cats.good_to_have_baseline:
        return GateOutcome(
            passed=False,
            reason=(
                f"GOOD_TO_HAVE failure count {len(report.good_to_have_failures)} "
                f"exceeds baseline {cats.good_to_have_baseline}"
            ),
        )
    return GateOutcome(passed=True, reason="ok")


# Cache invalidation hook for tests that author manifests inline and
# need a fresh load on next call.  Not part of the production API.
def _reset_cache_for_tests() -> None:
    with _cache_lock:
        _cache.clear()
