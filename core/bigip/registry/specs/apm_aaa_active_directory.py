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
            "apm_aaa_active_directory",
            module="apm",
            object_types=("aaa active-directory",),
        ),
        header_types=(("apm", "aaa active-directory"),),
        properties=(
            BigipPropertySpec(
                name="admin-encrypted-password",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="admin-name",
                value_type="string",
                required=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cleanup-cache",
                value_type="enum",
                allow_none=True,
                enum_values=("group", "kerberos", "none", "pso"),
                default="none",
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="domain", value_type="string", required=True, allow_none=True),
            BigipPropertySpec(
                name="domain-controller",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="domain-controllers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="ip",
                        value_type="string",
                        in_sections=("domain-controllers",),
                        shape_kind="ip-address",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="ip",
                value_type="string",
                in_sections=("domain-controllers",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="group-cache-ttl", value_type="integer", default="30"),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="padata", value_type="unknown", default="rc4-hmac"),
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
                default="none",
            ),
            BigipPropertySpec(name="pso-cache-ttl", value_type="integer", default="30"),
            BigipPropertySpec(name="timeout", value_type="integer", default="15"),
        ),
    )
