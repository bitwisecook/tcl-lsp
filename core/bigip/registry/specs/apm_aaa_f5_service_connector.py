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
            "apm_aaa_f5_service_connector",
            module="apm",
            object_types=("aaa f5-service-connector",),
        ),
        header_types=(("apm", "aaa f5-service-connector"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="customer-id", value_type="string"),
            BigipPropertySpec(name="customer-key", value_type="string"),
            BigipPropertySpec(
                name="dns-resolver",
                value_type="reference",
                references=("net_dns_resolver",),
            ),
            BigipPropertySpec(name="serverssl-profile", value_type="reference"),
            BigipPropertySpec(name="service-url", value_type="string"),
        ),
    )
