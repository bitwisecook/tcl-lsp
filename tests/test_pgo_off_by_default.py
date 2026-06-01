"""PGO must be off by default everywhere — it only runs through the
explicit, profile-driven entry points, never the normal optimiser."""

from __future__ import annotations

from compiler.optimiser._manager import find_optimisations
from compiler.pgo.reorder import REORDER_CODE
from shared.codes import optimisation_codes, pgo_codes
from shared.optimisation_profiles import OptimisationProfile, profile_to_disabled

# A chain that the PGO reorderer *would* flag given a profile.
DISPATCH = """\
when HTTP_REQUEST {
    if {[IP::remote_addr] eq "10.0.0.1"} {
        pool pool_a
    } elseif {[IP::remote_addr] eq "10.0.0.2"} {
        pool pool_b
    } elseif {[IP::remote_addr] eq "10.0.0.3"} {
        pool pool_c
    }
}
"""


def test_pgo_code_registered_but_not_an_optimisation_code() -> None:
    assert REORDER_CODE in pgo_codes()
    assert REORDER_CODE not in optimisation_codes()


def test_standard_optimiser_never_emits_pgo_codes() -> None:
    codes = {opt.code for opt in find_optimisations(DISPATCH)}
    assert REORDER_CODE not in codes


def test_no_optimisation_profile_enables_pgo() -> None:
    # ``full`` / ``aggressive`` enable *all* optimisation codes; none of
    # the profiles should ever leave a PGO code enabled (i.e. it must be
    # in every profile's disabled set — or, equivalently, never in the
    # optimisation universe at all).
    for profile in OptimisationProfile:
        disabled = profile_to_disabled(profile)
        # PGO codes are not optimisation codes, so they are neither enabled
        # nor disabled by the optimiser machinery — assert they never leak
        # into the enabled set.
        enabled = optimisation_codes() - disabled
        assert REORDER_CODE not in enabled
