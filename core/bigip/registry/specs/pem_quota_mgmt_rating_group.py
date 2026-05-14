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
            "pem_quota_mgmt_rating_group",
            module="pem",
            object_types=("quota-mgmt rating-group",),
        ),
        header_types=(("pem", "quota-mgmt rating-group"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="default-breach-action",
                value_type="enum",
                enum_values=("allow", "redirect", "terminate"),
            ),
            BigipPropertySpec(name="default-forwarding-endpoint", value_type="reference"),
            BigipPropertySpec(name="default-quota", value_type="unknown"),
            BigipPropertySpec(name="default-quota-holding-time", value_type="integer"),
            BigipPropertySpec(
                name="interval",
                value_type="integer",
                in_sections=("default-quota",),
            ),
            BigipPropertySpec(name="time", value_type="unknown", in_sections=("default-quota",)),
            BigipPropertySpec(
                name="consumption-time",
                value_type="unknown",
                in_sections=("default-quota", "time"),
            ),
            BigipPropertySpec(
                name="usage-time",
                value_type="unknown",
                in_sections=("default-quota", "time"),
            ),
            BigipPropertySpec(name="volume", value_type="unknown", in_sections=("default-quota",)),
            BigipPropertySpec(
                name="input-octets",
                value_type="unknown",
                in_sections=("default-quota", "volume"),
            ),
            BigipPropertySpec(
                name="output-octets",
                value_type="unknown",
                in_sections=("default-quota", "volume"),
            ),
            BigipPropertySpec(
                name="total-octets",
                value_type="unknown",
                in_sections=("default-quota", "volume"),
            ),
            BigipPropertySpec(name="default-threshold", value_type="integer"),
            BigipPropertySpec(name="default-validity-time", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="initial-quota-request", value_type="unknown"),
            BigipPropertySpec(
                name="interval",
                value_type="integer",
                in_sections=("initial-quota-request",),
            ),
            BigipPropertySpec(
                name="volume",
                value_type="unknown",
                in_sections=("initial-quota-request",),
            ),
            BigipPropertySpec(
                name="input-octets",
                value_type="unknown",
                in_sections=("initial-quota-request", "volume"),
            ),
            BigipPropertySpec(
                name="output-octets",
                value_type="unknown",
                in_sections=("initial-quota-request", "volume"),
            ),
            BigipPropertySpec(
                name="total-octets",
                value_type="unknown",
                in_sections=("initial-quota-request", "volume"),
            ),
            BigipPropertySpec(name="rating-group-id", value_type="integer"),
            BigipPropertySpec(
                name="request-on-install",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
        ),
    )
