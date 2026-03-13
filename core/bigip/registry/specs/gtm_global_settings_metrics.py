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
            BigipPropertySpec(name="default-probe-limit", value_type="integer"),
            BigipPropertySpec(name="hops-ttl", value_type="integer"),
            BigipPropertySpec(name="hops-packet-length", value_type="integer"),
            BigipPropertySpec(name="hops-sample-count", value_type="integer"),
            BigipPropertySpec(name="hops-timeout", value_type="integer"),
            BigipPropertySpec(name="inactive-ldns-ttl", value_type="integer"),
            BigipPropertySpec(name="ldns-update-interval", value_type="integer"),
            BigipPropertySpec(name="inactive-paths-ttl", value_type="integer"),
            BigipPropertySpec(name="max-synchronous-monitor-requests", value_type="integer"),
            BigipPropertySpec(name="metrics-caching", value_type="integer"),
            BigipPropertySpec(
                name="metrics-collection-protocols", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(name="path-ttl", value_type="integer"),
            BigipPropertySpec(name="paths-retry", value_type="integer"),
        ),
    )
