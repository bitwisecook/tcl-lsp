"""Unit tests for ``tests/external/_tcl9_categories.py``.

Validates the TOML loader against the three category files this stream
landed (expr-old / incr / string) and exercises the default-must_pass
gate rule.
"""

from __future__ import annotations

import pytest

from tests.external._tcl9_categories import (
    CATEGORIES_DIR,
    VALID_BUCKETS,
    GateDecision,
    category_for,
    gate_decision,
    load_stem,
    wasm_n_a_ids,
)


def test_categories_dir_exists() -> None:
    assert CATEGORIES_DIR.is_dir()
    assert (CATEGORIES_DIR / "README.md").is_file()


@pytest.mark.parametrize("stem", ["expr-old", "incr", "string"])
def test_landed_stems_parse(stem: str) -> None:
    cats = load_stem(stem)
    assert cats is not None, f"{stem}.toml missing"
    assert cats.bundle == f"{stem}.test"
    assert cats.total > 0
    # Every entry has a recognised bucket and a non-empty reason.
    for tid, entry in cats.entries.items():
        assert entry.id == tid
        assert entry.bucket in VALID_BUCKETS
        assert entry.reason.strip()


def test_load_unknown_stem_returns_none() -> None:
    assert load_stem("does-not-exist") is None


def test_default_bucket_is_must_pass() -> None:
    cats = load_stem("expr-old")
    assert cats is not None
    # Unknown ID falls through to must_pass.
    assert category_for(cats, "999.999") == "must_pass"
    # Known ID returns its triaged bucket.
    assert category_for(cats, "32.50") == "just_to_match_ctcl"
    assert category_for(cats, "34.10") == "must_pass"


def test_category_for_with_no_cats_is_must_pass() -> None:
    assert category_for(None, "anything") == "must_pass"


def test_gate_decision_blocks_on_unknown_failures() -> None:
    cats = load_stem("expr-old")
    decision = gate_decision(cats, ["32.50", "34.10", "999.999"])
    # 32.50 is just_to_match_ctcl — tracked, not blocking.
    # 34.10 is must_pass — blocking.
    # 999.999 is unknown → defaults to must_pass — blocking.
    assert "34.10" in decision.blocking_failures
    assert "999.999" in decision.blocking_failures
    assert "32.50" in decision.tracked_just_to_match_ctcl
    assert not decision.passes


def test_gate_decision_passes_when_all_failures_categorised() -> None:
    cats = load_stem("expr-old")
    # Drop the must_pass entries; the just_to_match_ctcl ones don't block.
    decision = gate_decision(cats, ["32.50", "32.53", "34.3", "36.16"])
    assert decision.blocking_failures == ()
    assert decision.passes
    assert len(decision.tracked_just_to_match_ctcl) == 4


def test_gate_decision_with_no_cats_blocks_everything() -> None:
    decision = gate_decision(None, ["1.1", "1.2"])
    assert sorted(decision.blocking_failures) == ["1.1", "1.2"]
    assert not decision.passes


def test_incr_landed_entry() -> None:
    cats = load_stem("incr")
    assert cats is not None
    assert "1.32" in cats.entries
    assert cats.entries["1.32"].bucket == "must_pass"


def test_string_has_no_entries_but_loads() -> None:
    # string.toml is a hand-off file with header notes only.  The
    # 332 residual failures default to must_pass and a follow-up
    # stream is expected to walk + bin them.
    cats = load_stem("string")
    assert cats is not None
    assert cats.entries == {}


def test_valid_buckets_includes_wasm_n_a() -> None:
    assert "wasm_n_a" in VALID_BUCKETS


def test_gate_decision_separates_wasm_n_a_from_just_to_match_ctcl() -> None:
    """``wasm_n_a`` and ``just_to_match_ctcl`` are reported in
    separate fields so the consumer can choose to *skip* the
    bytecode probes (filter out before running) versus *count*
    the wording differences."""
    import tomllib  # noqa: PLC0415

    from tests.external._tcl9_categories import _parse  # noqa: PLC0415

    raw = tomllib.loads(
        """
        [meta]
        bundle = "fake.test"
        total = 10
        stream = "test"

        [[entries]]
        id = "1.1"
        bucket = "wasm_n_a"
        reason = "disassemble probes Tcl bcc instructions"

        [[entries]]
        id = "1.2"
        bucket = "just_to_match_ctcl"
        reason = "error wording word-order"
        """
    )
    from pathlib import Path

    cats = _parse(raw, source=Path("inline"))
    decision = gate_decision(cats, ["1.1", "1.2", "1.3"])
    assert decision.tracked_wasm_n_a == ("1.1",)
    assert decision.tracked_just_to_match_ctcl == ("1.2",)
    # 1.3 is unknown → defaults to must_pass.
    assert decision.blocking_failures == ("1.3",)
    assert wasm_n_a_ids(cats) == ("1.1",)


def test_wasm_n_a_ids_with_no_cats() -> None:
    assert wasm_n_a_ids(None) == ()
