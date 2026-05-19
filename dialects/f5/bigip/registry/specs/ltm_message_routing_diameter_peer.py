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
            "ltm_message_routing_diameter_peer",
            module="ltm",
            object_types=("message-routing diameter peer",),
        ),
        header_types=(("ltm", "message-routing diameter peer"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auto-initialization",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="auto-initialization-interval",
                value_type="integer",
                default="5000ms",
            ),
            BigipPropertySpec(
                name="connection-mode",
                value_type="enum",
                enum_values=(
                    "per-blade",
                    "per-client",
                    "per-client-alternate-tmm",
                    "per-client-per-blade",
                    "per-client-per-tmm",
                    "per-peer",
                    "per-peer-alternate-tmm",
                    "per-tmm",
                ),
                default="per-peer",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="number-connections", value_type="integer", default="1"),
            BigipPropertySpec(
                name="pool",
                value_type="reference",
                references=(
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ),
            ),
            BigipPropertySpec(name="ratio", value_type="integer", default="1"),
            BigipPropertySpec(name="transport-config", value_type="unknown"),
        ),
    )
