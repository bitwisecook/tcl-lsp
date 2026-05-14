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
            "wom_endpoint_discovery",
            module="wom",
            object_types=("endpoint-discovery",),
        ),
        header_types=(("wom", "endpoint-discovery"),),
        properties=(
            BigipPropertySpec(
                name="auto-save",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="discoverable",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="discovered-endpoint",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="icmp-max-requests", value_type="integer"),
            BigipPropertySpec(name="icmp-min-backoff", value_type="integer"),
            BigipPropertySpec(name="icmp-num-retries", value_type="integer"),
            BigipPropertySpec(name="max-endpoint-count", value_type="integer"),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("disable", "enable-all", "enable-icmp", "enable-tcp"),
            ),
        ),
    )
