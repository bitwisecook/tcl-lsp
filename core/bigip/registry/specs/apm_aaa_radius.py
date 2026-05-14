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
            "apm_aaa_radius",
            module="apm",
            object_types=("aaa radius",),
        ),
        header_types=(("apm", "aaa radius"),),
        properties=(
            BigipPropertySpec(name="acct-port", value_type="integer"),
            BigipPropertySpec(name="address", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="auth-port", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("acct", "auth", "both")),
            BigipPropertySpec(name="nas-ip-address", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="nas-ipv6-address", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="pool",
                value_type="string",
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
            BigipPropertySpec(name="retries", value_type="integer"),
            BigipPropertySpec(name="secret", value_type="string"),
            BigipPropertySpec(
                name="service-type",
                value_type="enum",
                enum_values=(
                    "administrative",
                    "authenticate-only",
                    "call-check",
                    "callback-administrative",
                    "callback-framed",
                    "callback-login",
                    "callback-nas-promit",
                    "default",
                    "framed",
                    "login",
                    "nas-prompt",
                    "outbound",
                ),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="use-pool",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
