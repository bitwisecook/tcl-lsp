"""Unit tests for ``tests/external/_tcl9_categories.py``.

Validates the synthesised loader against the three category files this
stream landed (expr-old / incr / string), exercises the
default-must_pass gate rule, and covers the ``[baseline]`` cap and
overlap / unknown-key validation borrowed from the parallel
``tcl9-codegen-misc`` stream.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.external._tcl9_categories import (
    BUCKETS,
    DEFAULT_BUCKET,
    BucketReport,
    StemCategories,
    _reset_cache_for_tests,
    bucket_failures,
    gate,
    load_categories,
)

_CATEGORIES_DIR = Path(__file__).resolve().parents[1] / "baselines" / "tcl9-tcltest" / "categories"


@pytest.fixture(autouse=True)
def _clear_cache():
    """Each test loads from a clean cache so an inline-TOML test
    in this file can't pollute the on-disk read in another."""
    _reset_cache_for_tests()
    yield
    _reset_cache_for_tests()


def test_categories_dir_exists() -> None:
    assert _CATEGORIES_DIR.is_dir()
    assert (_CATEGORIES_DIR / "README.md").is_file()


def test_buckets_constant_includes_runtime_target_buckets() -> None:
    assert BUCKETS == (
        "must_pass",
        "good_to_have",
        "just_to_match_ctcl",
        "skip",
        "impossible_in_wasm_wasi",
        "impossible_in_wasm_browser",
    )
    assert DEFAULT_BUCKET == "must_pass"


@pytest.mark.parametrize("stem", ["expr-old", "incr"])
def test_landed_stems_parse(stem: str) -> None:
    cats = load_categories(stem)
    assert isinstance(cats, StemCategories)
    assert cats.stem == stem
    for tid, bucket in cats.test_to_bucket.items():
        assert bucket in BUCKETS, f"{stem}.toml: {tid!r} → bucket {bucket!r}"


def test_load_unknown_stem_is_empty_default() -> None:
    cats = load_categories("does-not-exist")
    assert cats.test_to_bucket == {}
    assert cats.trap_allowed is False
    assert cats.good_to_have_baseline == 0


def test_unlisted_id_is_must_pass() -> None:
    cats = load_categories("expr-old")
    assert cats.bucket_of("999.999") == "must_pass"
    assert cats.bucket_of("32.50") == "just_to_match_ctcl"
    assert cats.bucket_of("34.10") == "good_to_have"


def test_bucket_of_accepts_full_id_or_suffix() -> None:
    cats = load_categories("expr-old")
    assert cats.bucket_of("expr-old-32.50") == "just_to_match_ctcl"
    assert cats.bucket_of("32.50") == "just_to_match_ctcl"


def test_bucket_failures_partitions_correctly() -> None:
    report = bucket_failures(
        "expr-old",
        ["expr-old-32.50", "expr-old-34.10", "expr-old-999.999"],
    )
    assert report.must_pass_failures == ["expr-old-999.999"]
    assert report.good_to_have_failures == ["expr-old-34.10"]
    assert report.just_to_match_ctcl_failures == ["expr-old-32.50"]
    assert report.skip_failures == []
    assert report.total == 3


def test_bucket_failures_with_unknown_stem_blocks_everything() -> None:
    report = bucket_failures("does-not-exist", ["fake-1.1", "fake-1.2"])
    assert sorted(report.must_pass_failures) == ["fake-1.1", "fake-1.2"]
    assert report.good_to_have_failures == []


def test_incr_landed_entry() -> None:
    cats = load_categories("incr")
    assert cats.bucket_of("1.32") == "good_to_have"
    assert cats.good_to_have_baseline == 1


def test_skip_full_ids_for_landed_stems_is_empty() -> None:
    for stem in ("expr-old", "incr"):
        assert load_categories(stem).skip_full_ids() == []


def test_bucket_report_to_dict() -> None:
    report = BucketReport(
        must_pass_failures=["a"],
        good_to_have_failures=["b", "c"],
        just_to_match_ctcl_failures=[],
        skip_failures=["d"],
        impossible_in_wasm_wasi_failures=["e"],
        impossible_in_wasm_browser_failures=["f", "g"],
    )
    d = report.to_dict()
    assert d == {
        "must_pass": ["a"],
        "good_to_have": ["b", "c"],
        "just_to_match_ctcl": [],
        "skip": ["d"],
        "impossible_in_wasm_wasi": ["e"],
        "impossible_in_wasm_browser": ["f", "g"],
    }
    assert report.total == 7


def test_runtime_target_buckets_round_trip(tmp_path, monkeypatch) -> None:
    """Loading a TOML with the new buckets routes IDs correctly and
    surfaces the matching baseline counters."""
    inline = tmp_path / "rtt.toml"
    inline.write_text(
        """
        impossible_in_wasm_wasi = ["1.1"]
        impossible_in_wasm_browser = ["2.1", "2.2"]
        [baseline]
        impossible_in_wasm_wasi_failing = 1
        impossible_in_wasm_browser_failing = 2
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    cats = load_categories("rtt")
    assert cats.bucket_of("1.1") == "impossible_in_wasm_wasi"
    assert cats.bucket_of("2.1") == "impossible_in_wasm_browser"
    assert cats.bucket_of("999.9") == "must_pass"
    assert cats.impossible_in_wasm_wasi_baseline == 1
    assert cats.impossible_in_wasm_browser_baseline == 2
    # WASI helper returns just the wasi bucket (browser-stricter
    # IDs aren't WASI-impossible).
    assert cats.wasi_impossible_full_ids() == ["rtt-1.1"]
    # Browser is the strictly stricter sandbox, so the helper
    # composes the union of both buckets.
    assert sorted(cats.browser_impossible_full_ids()) == [
        "rtt-1.1",
        "rtt-2.1",
        "rtt-2.2",
    ]


def test_runtime_target_buckets_do_not_gate(tmp_path, monkeypatch) -> None:
    """Failures landing in either runtime-target-impossibility bucket
    must NOT fire the gate — they are tracked-only, like skip."""
    inline = tmp_path / "rtt2.toml"
    inline.write_text(
        """
        impossible_in_wasm_wasi = ["1.1"]
        impossible_in_wasm_browser = ["2.1"]
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    cats = load_categories("rtt2")
    report = bucket_failures("rtt2", ["rtt2-1.1", "rtt2-2.1"])
    assert report.must_pass_failures == []
    assert report.impossible_in_wasm_wasi_failures == ["rtt2-1.1"]
    assert report.impossible_in_wasm_browser_failures == ["rtt2-2.1"]
    outcome = gate(cats, report)
    assert outcome.passed
    assert outcome.reason == "ok"


def test_gate_passes_when_within_baseline() -> None:
    cats = load_categories("incr")
    # Single good_to_have failure, baseline = 1 — within cap.
    report = bucket_failures("incr", ["incr-1.32"])
    outcome = gate(cats, report)
    assert outcome.passed
    assert outcome.reason == "ok"


def test_gate_fails_on_must_pass() -> None:
    cats = load_categories("incr")
    report = bucket_failures("incr", ["incr-99.99"])  # unknown → must_pass
    outcome = gate(cats, report)
    assert not outcome.passed
    assert "MUST_PASS" in outcome.reason


def test_gate_fails_on_good_to_have_growth(tmp_path, monkeypatch) -> None:
    """Two good_to_have failures with baseline=1 must fire the gate."""
    inline = tmp_path / "synth.toml"
    inline.write_text(
        """
        good_to_have = ["1.1", "1.2"]
        just_to_match_ctcl = []
        skip = []
        [baseline]
        good_to_have_failing = 1
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    cats = load_categories("synth")
    report = BucketReport(
        good_to_have_failures=["synth-1.1", "synth-1.2"],
    )
    outcome = gate(cats, report)
    assert not outcome.passed
    assert "GOOD_TO_HAVE failure count 2 exceeds baseline 1" in outcome.reason


def test_loader_rejects_overlap(tmp_path, monkeypatch) -> None:
    inline = tmp_path / "bad.toml"
    inline.write_text(
        """
        good_to_have = ["1.1"]
        just_to_match_ctcl = ["1.1"]
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    with pytest.raises(RuntimeError, match="listed in both"):
        load_categories("bad")


def test_loader_rejects_unknown_top_level_key(tmp_path, monkeypatch) -> None:
    inline = tmp_path / "typo.toml"
    inline.write_text(
        """
        goodto_have = ["1.1"]
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    with pytest.raises(RuntimeError, match="unknown top-level keys"):
        load_categories("typo")


def test_loader_rejects_unknown_baseline_key(tmp_path, monkeypatch) -> None:
    inline = tmp_path / "bad.toml"
    inline.write_text(
        """
        good_to_have = ["1.1"]
        [baseline]
        good_to_have_failing = 1
        good_to_haves_failing = 1
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    with pytest.raises(RuntimeError, match="unknown \\[baseline\\] keys"):
        load_categories("bad")


def test_loader_rejects_baseline_exceeding_listed_count(tmp_path, monkeypatch) -> None:
    inline = tmp_path / "bad.toml"
    inline.write_text(
        """
        good_to_have = ["1.1"]
        [baseline]
        good_to_have_failing = 5
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    with pytest.raises(RuntimeError, match="exceeds the number of listed"):
        load_categories("bad")


def test_trap_allowed_round_trip(tmp_path, monkeypatch) -> None:
    inline = tmp_path / "trap.toml"
    inline.write_text("trap_allowed = true\n", encoding="utf-8")
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    cats = load_categories("trap")
    assert cats.trap_allowed is True


def test_skip_full_ids_round_trips_with_stem_prefix(tmp_path, monkeypatch) -> None:
    inline = tmp_path / "skipper.toml"
    inline.write_text(
        """
        skip = ["1.1", "2.2"]
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr("tests.external._tcl9_categories._CATEGORIES_DIR", tmp_path)
    _reset_cache_for_tests()
    cats = load_categories("skipper")
    assert cats.skip_full_ids() == ["skipper-1.1", "skipper-2.2"]
