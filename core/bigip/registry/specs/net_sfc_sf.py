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
            "net_sfc_sf",
            module="net",
            object_types=("sfc sf",),
        ),
        header_types=(("net", "sfc sf"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="egress-interface", value_type="string", allow_none=True),
            BigipPropertySpec(name="ingress-interface", value_type="string", allow_none=True),
            BigipPropertySpec(name="ip-address", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="nsh-aware",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="pool-name",
                value_type="reference",
                allow_none=True,
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
            BigipPropertySpec(
                name="virtual-name",
                value_type="reference",
                allow_none=True,
                references=(
                    "analytics_ssl_orchestrator_service_virtual_report",
                    "analytics_ssl_orchestrator_service_virtual_scheduled_report",
                    "analytics_virtual_report",
                    "analytics_virtual_scheduled_report",
                    "ltm_monitor_virtual_location",
                    "ltm_virtual",
                    "ltm_virtual_address",
                    "security_dos_virtual",
                    "security_protocol_inspection_virtual_servers",
                    "vcmp_virtual_disk",
                    "vcmp_virtual_disk_template",
                ),
            ),
        ),
    )
