"""Optimisation profiles — shared tier definitions for all surfaces.

Profiles group the 28 optimisation passes (O100-O127) into named tiers
ranging from maximum readability to aggressive multi-pass optimisation.
All integration points (LSP editor, CLI, AI skills, MCP tools, chat
commands) resolve a profile name into a ``disabled_optimisations`` set
that plugs into the existing per-code filtering mechanism.

Profiles
--------
- **off**          — All optimisations disabled.
- **readability**  — Idiomatic rewrites only (no code removal or restructuring).
- **standard**     — Readability + constant folding and pattern recognition.
- **full**         — All 28 passes, single pass (previous default).
- **aggressive**   — All 28 passes, multi-pass to fixpoint.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from .codes import optimisation_codes


class OptimisationProfile(Enum):
    """Named optimisation tiers."""

    OFF = "off"
    READABILITY = "readability"
    STANDARD = "standard"
    FULL = "full"
    AGGRESSIVE = "aggressive"


@dataclass(frozen=True, slots=True)
class ProfileSpec:
    """Defines which optimisation codes are enabled and pass behaviour."""

    name: str
    description: str
    enabled_codes: frozenset[str]
    multi_pass: bool = False
    max_iterations: int = 1


# ---------------------------------------------------------------------------
# Code categories
# ---------------------------------------------------------------------------

#: Idiomatic rewrites — improve clarity without removing or restructuring code.
READABILITY_CODES: frozenset[str] = frozenset(
    {
        "O111",  # Brace expression performance hints
        "O114",  # incr idiom
        "O115",  # Redundant nested [expr]
        "O117",  # string length == 0 simplification
        "O120",  # eq/ne for string comparisons
    }
)

#: Constant folding and expression simplification.
CONSTANT_FOLDING_CODES: frozenset[str] = frozenset(
    {
        "O100",  # Propagate constant variables
        "O101",  # Fold constant integer expressions
        "O102",  # Fold constant [expr {...}] substitutions
        "O103",  # Fold static procedure calls (ICIP)
        "O105",  # GVN/CSE — redundant computation detection
        "O110",  # Canonicalise expressions (InstCombine)
        "O113",  # Strength reduction
        "O116",  # Fold constant [list]
        "O118",  # Fold constant [lindex]
    }
)

#: Pattern recognition and packing.
PATTERN_CODES: frozenset[str] = frozenset(
    {
        "O104",  # Fold string build chains
        "O119",  # Pack consecutive set literals into lassign/foreach
    }
)

#: Dead code and dead store elimination.
DCE_CODES: frozenset[str] = frozenset(
    {
        "O107",  # Unreachable dead code
        "O108",  # Aggressive DCE (ADCE)
        "O109",  # Dead store elimination (DSE)
        "O112",  # Constant-condition block elimination (SCCP)
        "O124",  # Unused iRule procs
        "O126",  # Unused variable assignments
    }
)

#: Code motion — moving or inlining assignments.
CODE_MOTION_CODES: frozenset[str] = frozenset(
    {
        "O106",  # Loop-invariant code motion (LICM)
        "O125",  # Code sinking
        "O127",  # Inline single-use variable
    }
)

#: Recursion transforms.
RECURSION_CODES: frozenset[str] = frozenset(
    {
        "O121",  # Tail-call rewrite
        "O122",  # Tail-recursion to while loop
        "O123",  # Accumulator introduction hint
    }
)

# ---------------------------------------------------------------------------
# Profile specifications
# ---------------------------------------------------------------------------

#: All optimisation codes (populated lazily to avoid import-order issues).
_all_codes_cache: frozenset[str] | None = None


def _all_codes() -> frozenset[str]:
    global _all_codes_cache
    if _all_codes_cache is None:
        _all_codes_cache = optimisation_codes()
    return _all_codes_cache


_STANDARD_CODES = READABILITY_CODES | CONSTANT_FOLDING_CODES | PATTERN_CODES


def _full_codes() -> frozenset[str]:
    return _all_codes()


PROFILES: dict[OptimisationProfile, ProfileSpec] = {
    OptimisationProfile.OFF: ProfileSpec(
        name="off",
        description="All optimisations disabled.",
        enabled_codes=frozenset(),
    ),
    OptimisationProfile.READABILITY: ProfileSpec(
        name="readability",
        description="Readability improvements only — idiomatic rewrites, no code removal.",
        enabled_codes=READABILITY_CODES,
    ),
    OptimisationProfile.STANDARD: ProfileSpec(
        name="standard",
        description="Readability + constant folding and pattern recognition.",
        enabled_codes=_STANDARD_CODES,
    ),
    # NOTE: ``full`` and ``aggressive`` use _all_codes() at resolution time
    # (via profile_to_disabled) so they automatically include any future codes.
}

#: Valid profile name strings (for CLI/config validation).
PROFILE_NAMES: frozenset[str] = frozenset(p.value for p in OptimisationProfile)

#: Default profile for editor diagnostics (real-time squiggles).
DEFAULT_EDITOR_PROFILE = OptimisationProfile.READABILITY

#: Default profile for explicit optimisation actions (CLI, chat, MCP, AI).
DEFAULT_ACTION_PROFILE = OptimisationProfile.FULL


def profile_spec(profile: OptimisationProfile) -> ProfileSpec:
    """Return the spec for *profile*, handling dynamic code sets."""
    if profile is OptimisationProfile.FULL:
        return ProfileSpec(
            name="full",
            description="All optimisations enabled (single pass).",
            enabled_codes=_all_codes(),
        )
    if profile is OptimisationProfile.AGGRESSIVE:
        return ProfileSpec(
            name="aggressive",
            description="All optimisations with multi-pass to fixpoint.",
            enabled_codes=_all_codes(),
            multi_pass=True,
            max_iterations=5,
        )
    return PROFILES[profile]


def profile_from_name(name: str) -> OptimisationProfile:
    """Parse a profile name string (case-insensitive).

    Raises :class:`ValueError` for unknown names.
    """
    key = name.strip().lower()
    try:
        return OptimisationProfile(key)
    except ValueError:
        valid = ", ".join(sorted(PROFILE_NAMES))
        raise ValueError(
            f"Unknown optimisation profile {name!r}. Valid profiles: {valid}"
        ) from None


def profile_to_disabled(profile: OptimisationProfile) -> frozenset[str]:
    """Return the set of codes to *disable* for the given profile."""
    spec = profile_spec(profile)
    return _all_codes() - spec.enabled_codes


def resolve_profile(
    name: str | None,
    *,
    default: OptimisationProfile = DEFAULT_EDITOR_PROFILE,
    explicit_enable: set[str] | None = None,
    explicit_disable: set[str] | None = None,
) -> tuple[frozenset[str], bool, int]:
    """Resolve a profile name + per-code overrides into filter parameters.

    Returns ``(disabled_codes, multi_pass, max_iterations)``.

    Resolution order:
    1. Start from the profile's disabled set.
    2. Remove any codes in *explicit_enable* (force them on).
    3. Add any codes in *explicit_disable* (force them off).
    """
    profile = profile_from_name(name) if name else default
    spec = profile_spec(profile)
    disabled = set(_all_codes() - spec.enabled_codes)

    if explicit_enable:
        disabled -= explicit_enable
    if explicit_disable:
        disabled |= explicit_disable

    return frozenset(disabled), spec.multi_pass, spec.max_iterations
