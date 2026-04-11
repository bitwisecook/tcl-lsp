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
            "gtm_pool_naptr",
            module="gtm",
            object_types=("pool naptr",),
        ),
        header_types=(("gtm", "pool naptr"),),
        properties=(
            BigipPropertySpec(name="alternate-mode", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dynamic", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="fallback-mode", value_type="string"),
            BigipPropertySpec(name="load-balancing-mode", value_type="string"),
            BigipPropertySpec(
                name="manual-resume", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-answers-returned", value_type="integer"),
            BigipPropertySpec(name="members", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="flags", value_type="string"),
            BigipPropertySpec(name="member-order", value_type="integer"),
            BigipPropertySpec(name="order", value_type="integer"),
            BigipPropertySpec(name="preference", value_type="integer"),
            BigipPropertySpec(name="ratio", value_type="integer"),
            BigipPropertySpec(name="service", value_type="string"),
            BigipPropertySpec(name="metadata", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(name="qos-hit-ratio", value_type="integer"),
            BigipPropertySpec(name="qos-hops", value_type="integer"),
            BigipPropertySpec(name="qos-kilobytes-second", value_type="integer"),
            BigipPropertySpec(name="qos-lcs", value_type="integer"),
            BigipPropertySpec(name="qos-packet-rate", value_type="integer"),
            BigipPropertySpec(name="qos-rtt", value_type="integer"),
            BigipPropertySpec(name="qos-topology", value_type="integer"),
            BigipPropertySpec(name="qos-vs-capacity", value_type="integer"),
            BigipPropertySpec(name="qos-vs-score", value_type="integer"),
            BigipPropertySpec(name="ttl", value_type="integer"),
            BigipPropertySpec(
                name="verify-member-availability",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
