from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_profile_pcp",
            module="ltm",
            object_types=("profile pcp",),
        ),
        header_types=(("ltm", "profile pcp"),),
        properties=(
            BigipPropertySpec(
                name="announce-after-failover",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="announce-multicast", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_pcp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="map-filter-limit", value_type="integer"),
            BigipPropertySpec(name="map-limit-per-client", value_type="integer"),
            BigipPropertySpec(name="map-recycle-delay", value_type="integer"),
            BigipPropertySpec(name="max-mapping-lifetime", value_type="integer"),
            BigipPropertySpec(name="min-mapping-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="rule", value_type="reference", allow_none=True, references=("ltm_rule",)
            ),
            BigipPropertySpec(
                name="third-party-option", value_type="enum", enum_values=("enabled", "disabled")
            ),
        ),
    )
