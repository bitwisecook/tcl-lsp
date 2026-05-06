"""Loader for ``tests/baselines/tcl9-tcltest/categories/<stem>.toml``.

The TOML files bin individual tcltest failures into one of four
buckets:

* ``must_pass`` — gates the sweep; count must be 0.
* ``good_to_have`` — tracked but doesn't gate; count cannot grow.
* ``wasm_n_a`` — **skipped**: tests probing exact Tcl VM bytecode
  that can't pass in principle on a wasm-codegen runtime.
* ``just_to_match_ctcl`` — counted for visibility, never gates;
  non-semantic implementation-detail differences (error-wording
  variations that are functionally equivalent, dict shimmer
  ordering, etc.).

The default for any failing test that has no explicit entry is
``must_pass``: that forces explicit triage on every new failure.

Schema (see ``categories/README.md`` for the long form):

    [meta]
    bundle = "expr-old.test"
    total = 461
    stream = "tcl9-arith-wording"

    [[entries]]
    id = "32.50"
    bucket = "just_to_match_ctcl"
    reason = "..."

This module only loads + validates the TOML; wiring the buckets into
the actual sweep gate is the consumer's job (see
:func:`category_for` for the lookup primitive and
:func:`gate_decision` for the must_pass-by-default rule).
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

if sys.version_info >= (3, 11):
    import tomllib
else:  # pragma: no cover — repo runs on 3.11+
    import tomli as tomllib  # type: ignore[no-redef]

VALID_BUCKETS = ("must_pass", "good_to_have", "wasm_n_a", "just_to_match_ctcl")

CATEGORIES_DIR = (
    Path(__file__).parent.parent / "baselines" / "tcl9-tcltest" / "categories"
)


@dataclass(frozen=True, slots=True)
class CategoryEntry:
    """A single triaged test failure."""

    id: str
    bucket: str
    reason: str


@dataclass(frozen=True, slots=True)
class StemCategories:
    """Parsed contents of one ``<stem>.toml`` file.

    ``entries`` is keyed by test ID (the suffix after ``<stem>-``,
    e.g. ``"32.50"``) so :func:`category_for` is O(1).
    """

    bundle: str
    total: int
    stream: str
    entries: dict[str, CategoryEntry] = field(default_factory=dict)


def load_stem(stem: str) -> StemCategories | None:
    """Return the parsed categories for ``<stem>.test`` or ``None``.

    A missing file is *not* an error — the gate falls through to the
    default-``must_pass`` rule for every failure in that bundle.
    """
    path = CATEGORIES_DIR / f"{stem}.toml"
    if not path.exists():
        return None
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    return _parse(raw, source=path)


def _parse(raw: dict[str, object], *, source: Path) -> StemCategories:
    meta = raw.get("meta")
    if not isinstance(meta, dict):
        raise ValueError(f"{source}: missing [meta] table")
    bundle = meta.get("bundle")
    total = meta.get("total")
    stream = meta.get("stream")
    if not isinstance(bundle, str) or not bundle.endswith(".test"):
        raise ValueError(f"{source}: meta.bundle must be \"<stem>.test\"")
    if not isinstance(total, int) or total <= 0:
        raise ValueError(f"{source}: meta.total must be a positive int")
    if not isinstance(stream, str):
        raise ValueError(f"{source}: meta.stream must be a string")

    entries: dict[str, CategoryEntry] = {}
    raw_entries = raw.get("entries", [])
    if not isinstance(raw_entries, list):
        raise ValueError(f"{source}: entries must be an array of tables")
    for i, e in enumerate(raw_entries):
        if not isinstance(e, dict):
            raise ValueError(f"{source}: entries[{i}] must be a table")
        eid = e.get("id")
        bucket = e.get("bucket")
        reason = e.get("reason")
        if not isinstance(eid, str) or not eid:
            raise ValueError(f"{source}: entries[{i}].id must be a non-empty string")
        if bucket not in VALID_BUCKETS:
            raise ValueError(
                f"{source}: entries[{i}].bucket={bucket!r} must be one of {VALID_BUCKETS}"
            )
        if not isinstance(reason, str) or not reason.strip():
            raise ValueError(f"{source}: entries[{i}].reason must be a non-empty string")
        if eid in entries:
            raise ValueError(f"{source}: duplicate entry id {eid!r}")
        entries[eid] = CategoryEntry(id=eid, bucket=bucket, reason=reason)
    return StemCategories(
        bundle=bundle,
        total=total,
        stream=stream,
        entries=entries,
    )


def category_for(cats: StemCategories | None, test_id: str) -> str:
    """Bucket assignment for a single failing test ID.

    The default — when ``cats`` is ``None`` or the ID isn't listed —
    is ``must_pass``.  That's the explicit-triage rule: a new failure
    blocks the gate until someone classifies it.
    """
    if cats is None:
        return "must_pass"
    entry = cats.entries.get(test_id)
    if entry is None:
        return "must_pass"
    return entry.bucket


@dataclass(frozen=True, slots=True)
class GateDecision:
    """Result of :func:`gate_decision`.

    ``blocking_failures`` is the must_pass set that gates the sweep
    (empty = pass).  Other tuples are reported for visibility:

    * ``tracked_good_to_have`` — features we intend to ship but
      haven't yet.
    * ``tracked_wasm_n_a`` — bytecode-shape probes we treat as
      skipped because a wasm-codegen runtime can't satisfy them
      in principle.  These should be filtered out of the failing
      set entirely by the consumer (e.g. via tcltest constraint
      injection) once the IDs are known.
    * ``tracked_just_to_match_ctcl`` — non-semantic wording /
      ordering / display differences.
    """

    blocking_failures: tuple[str, ...]
    tracked_good_to_have: tuple[str, ...]
    tracked_wasm_n_a: tuple[str, ...]
    tracked_just_to_match_ctcl: tuple[str, ...]

    @property
    def passes(self) -> bool:
        return len(self.blocking_failures) == 0


def gate_decision(
    cats: StemCategories | None, failing_ids: Iterable[str]
) -> GateDecision:
    """Bucket the *failing_ids* from one bundle run.

    Failing IDs not in the categories file (or any ID when ``cats``
    is ``None``) land in ``must_pass`` and block the gate.  Listed
    ``good_to_have`` / ``wasm_n_a`` / ``just_to_match_ctcl`` IDs are
    returned for reporting but don't block.
    """
    blocking: list[str] = []
    good: list[str] = []
    wasm_na: list[str] = []
    ctcl: list[str] = []
    for tid in failing_ids:
        bucket = category_for(cats, tid)
        if bucket == "must_pass":
            blocking.append(tid)
        elif bucket == "good_to_have":
            good.append(tid)
        elif bucket == "wasm_n_a":
            wasm_na.append(tid)
        else:
            ctcl.append(tid)
    return GateDecision(
        blocking_failures=tuple(blocking),
        tracked_good_to_have=tuple(good),
        tracked_wasm_n_a=tuple(wasm_na),
        tracked_just_to_match_ctcl=tuple(ctcl),
    )


def wasm_n_a_ids(cats: StemCategories | None) -> tuple[str, ...]:
    """Return the test IDs marked ``wasm_n_a`` in *cats*.

    Consumers that want to filter these out of the running test set
    entirely (e.g. by adding a tcltest ``-constraints`` injection or
    a pre-execution skip list) can read this directly without
    re-walking the entry dict.
    """
    if cats is None:
        return ()
    return tuple(
        e.id for e in cats.entries.values() if e.bucket == "wasm_n_a"
    )
