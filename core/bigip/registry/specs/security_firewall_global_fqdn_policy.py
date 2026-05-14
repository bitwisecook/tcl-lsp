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
            "security_firewall_global_fqdn_policy",
            module="security",
            object_types=("firewall global-fqdn-policy",),
        ),
        header_types=(("security", "firewall global-fqdn-policy"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dns-resolver",
                value_type="reference",
                allow_none=True,
                references=(
                    "analytics_dns_cache_resolver_report",
                    "ltm_dns_cache_resolver",
                    "ltm_dns_cache_validating_resolver",
                    "net_dns_resolver",
                ),
            ),
            BigipPropertySpec(name="recursive", value_type="unknown"),
            BigipPropertySpec(name="refresh-interval", value_type="integer"),
        ),
    )
