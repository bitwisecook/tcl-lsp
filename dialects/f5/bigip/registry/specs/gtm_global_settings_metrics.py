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
            "gtm_global_settings_metrics",
            module="gtm",
            object_types=("global-settings metrics",),
        ),
        header_types=(("gtm", "global-settings metrics"),),
        properties=(
            BigipPropertySpec(name="default-probe-limit", value_type="integer", default="12"),
            BigipPropertySpec(name="hops-packet-length", value_type="integer", default="64"),
            BigipPropertySpec(name="hops-sample-count", value_type="integer", default="3"),
            BigipPropertySpec(name="hops-timeout", value_type="integer", default="3"),
            BigipPropertySpec(name="hops-ttl", value_type="integer", default="604800"),
            BigipPropertySpec(
                name="inactive-ldns-ttl",
                value_type="integer",
                default="2419200 (28 days)",
            ),
            BigipPropertySpec(
                name="inactive-paths-ttl",
                value_type="integer",
                default="604800 (7 days)",
            ),
            BigipPropertySpec(
                name="ldns-update-interval",
                value_type="integer",
                default="20 seconds",
            ),
            BigipPropertySpec(
                name="max-synchronous-monitor-requests",
                value_type="integer",
                default="20",
            ),
            BigipPropertySpec(
                name="metrics-caching",
                value_type="integer",
                min_value=0,
                max_value=604800,
                default="3600",
            ),
            BigipPropertySpec(
                name="metrics-collection-protocols",
                value_type="unknown",
                allow_none=True,
            ),
            BigipPropertySpec(name="path-ttl", value_type="integer", default="2400"),
            BigipPropertySpec(name="paths-retry", value_type="integer", default="120"),
        ),
    )
