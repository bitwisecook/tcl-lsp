"""Tests for the optimisation profile system."""

from __future__ import annotations

import pytest

import core.common.codes_all  # noqa: F401  # trigger all code registrations
from core.common.codes import optimisation_codes
from core.common.optimisation_profiles import (
    CODE_MOTION_CODES,
    CONSTANT_FOLDING_CODES,
    DCE_CODES,
    DEFAULT_ACTION_PROFILE,
    DEFAULT_EDITOR_PROFILE,
    PATTERN_CODES,
    READABILITY_CODES,
    RECURSION_CODES,
    OptimisationProfile,
    profile_from_name,
    profile_spec,
    profile_to_disabled,
    resolve_profile,
)

# ---------------------------------------------------------------------------
# Category sets
# ---------------------------------------------------------------------------


class TestCategoryCoverage:
    """Verify category frozensets are consistent with the code registry."""

    def test_categories_cover_all_codes(self):
        all_codes = optimisation_codes()
        union = (
            READABILITY_CODES
            | CONSTANT_FOLDING_CODES
            | PATTERN_CODES
            | DCE_CODES
            | CODE_MOTION_CODES
            | RECURSION_CODES
        )
        assert union == all_codes, f"Missing from categories: {all_codes - union}"

    def test_categories_are_disjoint(self):
        categories = [
            READABILITY_CODES,
            CONSTANT_FOLDING_CODES,
            PATTERN_CODES,
            DCE_CODES,
            CODE_MOTION_CODES,
            RECURSION_CODES,
        ]
        for i, a in enumerate(categories):
            for b in categories[i + 1 :]:
                overlap = a & b
                assert not overlap, f"Categories overlap: {overlap}"

    def test_no_empty_categories(self):
        for name, cat in [
            ("READABILITY_CODES", READABILITY_CODES),
            ("CONSTANT_FOLDING_CODES", CONSTANT_FOLDING_CODES),
            ("PATTERN_CODES", PATTERN_CODES),
            ("DCE_CODES", DCE_CODES),
            ("CODE_MOTION_CODES", CODE_MOTION_CODES),
            ("RECURSION_CODES", RECURSION_CODES),
        ]:
            assert cat, f"{name} is empty"


# ---------------------------------------------------------------------------
# Profile specs
# ---------------------------------------------------------------------------


class TestProfileSpec:
    """Verify each profile's enabled codes and parameters."""

    def test_off_disables_everything(self):
        spec = profile_spec(OptimisationProfile.OFF)
        assert spec.enabled_codes == frozenset()
        assert not spec.multi_pass
        assert spec.max_iterations == 1

    def test_readability_codes(self):
        spec = profile_spec(OptimisationProfile.READABILITY)
        assert spec.enabled_codes == READABILITY_CODES
        assert not spec.multi_pass
        assert len(spec.enabled_codes) == 5

    def test_standard_includes_readability(self):
        spec = profile_spec(OptimisationProfile.STANDARD)
        assert READABILITY_CODES <= spec.enabled_codes
        assert CONSTANT_FOLDING_CODES <= spec.enabled_codes
        assert PATTERN_CODES <= spec.enabled_codes
        # Does not include DCE, code motion, or recursion.
        assert not (DCE_CODES & spec.enabled_codes)
        assert not (CODE_MOTION_CODES & spec.enabled_codes)
        assert not (RECURSION_CODES & spec.enabled_codes)
        assert not spec.multi_pass

    def test_full_enables_all(self):
        spec = profile_spec(OptimisationProfile.FULL)
        assert spec.enabled_codes == optimisation_codes()
        assert not spec.multi_pass

    def test_aggressive_enables_all_with_multipass(self):
        spec = profile_spec(OptimisationProfile.AGGRESSIVE)
        assert spec.enabled_codes == optimisation_codes()
        assert spec.multi_pass
        assert spec.max_iterations >= 3

    def test_standard_code_count(self):
        spec = profile_spec(OptimisationProfile.STANDARD)
        assert len(spec.enabled_codes) == 16


# ---------------------------------------------------------------------------
# profile_from_name
# ---------------------------------------------------------------------------


class TestProfileFromName:
    def test_valid_names(self):
        for name in ("off", "readability", "standard", "full", "aggressive"):
            p = profile_from_name(name)
            assert p.value == name

    def test_case_insensitive(self):
        assert profile_from_name("FULL") is OptimisationProfile.FULL
        assert profile_from_name("Readability") is OptimisationProfile.READABILITY

    def test_whitespace_stripped(self):
        assert profile_from_name("  full  ") is OptimisationProfile.FULL

    def test_invalid_name_raises(self):
        with pytest.raises(ValueError, match="Unknown optimisation profile"):
            profile_from_name("turbo")


# ---------------------------------------------------------------------------
# profile_to_disabled
# ---------------------------------------------------------------------------


class TestProfileToDisabled:
    def test_off_disables_all(self):
        disabled = profile_to_disabled(OptimisationProfile.OFF)
        assert disabled == optimisation_codes()

    def test_full_disables_none(self):
        disabled = profile_to_disabled(OptimisationProfile.FULL)
        assert disabled == frozenset()

    def test_readability_complement(self):
        disabled = profile_to_disabled(OptimisationProfile.READABILITY)
        assert disabled == optimisation_codes() - READABILITY_CODES


# ---------------------------------------------------------------------------
# resolve_profile
# ---------------------------------------------------------------------------


class TestResolveProfile:
    def test_default_profile(self):
        disabled, multi, max_iter = resolve_profile(None)
        assert disabled == profile_to_disabled(DEFAULT_EDITOR_PROFILE)

    def test_explicit_name(self):
        disabled, multi, max_iter = resolve_profile("full")
        assert disabled == frozenset()
        assert not multi

    def test_aggressive_returns_multipass(self):
        disabled, multi, max_iter = resolve_profile("aggressive")
        assert multi
        assert max_iter >= 3

    def test_explicit_enable_overrides_profile(self):
        disabled, _, _ = resolve_profile("readability", explicit_enable={"O100"})
        assert "O100" not in disabled

    def test_explicit_disable_overrides_profile(self):
        disabled, _, _ = resolve_profile("full", explicit_disable={"O109"})
        assert "O109" in disabled

    def test_invalid_name_uses_default(self):
        # resolve_profile raises ValueError for invalid names (from profile_from_name).
        with pytest.raises(ValueError):
            resolve_profile("nonexistent")

    def test_custom_default(self):
        disabled, _, _ = resolve_profile(None, default=OptimisationProfile.FULL)
        assert disabled == frozenset()


# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------


class TestDefaults:
    def test_editor_default_is_readability(self):
        assert DEFAULT_EDITOR_PROFILE is OptimisationProfile.READABILITY

    def test_action_default_is_full(self):
        assert DEFAULT_ACTION_PROFILE is OptimisationProfile.FULL
