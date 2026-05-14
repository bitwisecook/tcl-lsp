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
            "apm_aaa_crldp",
            module="apm",
            object_types=("aaa crldp",),
        ),
        header_types=(("apm", "aaa crldp"),),
        properties=(
            BigipPropertySpec(name="address", value_type="unknown"),
            BigipPropertySpec(
                name="allow-nullcrl",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="base-dn",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="cache-expire", value_type="integer", allow_none=True),
            BigipPropertySpec(name="connection-timeout", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="description",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
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
            BigipPropertySpec(name="port", value_type="integer", allow_none=True),
            BigipPropertySpec(name="reverse-dn", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(name="use-issuer", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(
                name="use-pool",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="verify-sig", value_type="enum", enum_values=("false", "true")),
        ),
    )
