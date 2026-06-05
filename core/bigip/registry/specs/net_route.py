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
            "net_route",
            module="net",
            object_types=("route",),
        ),
        header_types=(("net", "route"),),
        properties=(
            BigipPropertySpec(name="blackhole", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="gw",
                value_type="string",
                references=("net_self",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="interface",
                value_type="reference",
                references=("net_interface", "net_interface_cos", "net_interface_ddm"),
            ),
            BigipPropertySpec(name="mtu", value_type="integer"),
            BigipPropertySpec(name="network", value_type="string", shape_kind="ip-address"),
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
        ),
    )
