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
            "ltm_monitor_module_score",
            module="ltm",
            object_types=("monitor module-score",),
        ),
        header_types=(("ltm", "monitor module-score"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_module_score",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
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
            BigipPropertySpec(name="snmp-community", value_type="string"),
            BigipPropertySpec(name="snmp-ip-address", value_type="string", allow_none=True),
            BigipPropertySpec(name="snmp-port", value_type="unknown"),
            BigipPropertySpec(name="snmp-version", value_type="string"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
        ),
    )
