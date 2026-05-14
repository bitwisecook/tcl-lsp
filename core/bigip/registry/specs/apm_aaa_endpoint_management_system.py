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
            "apm_aaa_endpoint_management_system",
            module="apm",
            object_types=("aaa endpoint-management-system",),
        ),
        header_types=(("apm", "aaa endpoint-management-system"),),
        properties=(
            BigipPropertySpec(name="access-key", value_type="string", allow_none=True),
            BigipPropertySpec(name="app-version", value_type="string", allow_none=True),
            BigipPropertySpec(name="application-id", value_type="string", allow_none=True),
            BigipPropertySpec(name="billing-id", value_type="string", allow_none=True),
            BigipPropertySpec(name="client-id", value_type="string", allow_none=True),
            BigipPropertySpec(name="client-secret", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="dns-resolver",
                value_type="reference",
                allow_none=True,
                references=("net_dns_resolver",),
            ),
            BigipPropertySpec(name="fqdn", value_type="string"),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="mdm-token", value_type="string", allow_none=True),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="platform", value_type="string", allow_none=True),
            BigipPropertySpec(name="port", value_type="unknown"),
            BigipPropertySpec(name="serverssl-profile", value_type="reference"),
            BigipPropertySpec(name="sync-interval", value_type="integer", allow_none=True),
            BigipPropertySpec(name="tenant-id", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("airwatch", "maas360", "ms-intune"),
            ),
            BigipPropertySpec(name="username", value_type="string"),
        ),
    )
