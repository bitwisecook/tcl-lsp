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
            "pem_service_chain_endpoint",
            module="pem",
            object_types=("service-chain-endpoint",),
        ),
        header_types=(("pem", "service-chain-endpoint"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="service-endpoints",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("service-endpoints",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="forwarding-endpoint",
                value_type="unknown",
                in_sections=("service-endpoints",),
            ),
            BigipPropertySpec(
                name="from-vlan",
                value_type="reference",
                in_sections=("service-endpoints",),
            ),
            BigipPropertySpec(
                name="http-adapt-service",
                value_type="unknown",
                in_sections=("service-endpoints",),
            ),
            BigipPropertySpec(
                name="icap-type",
                value_type="enum",
                in_sections=("service-endpoints",),
                allow_none=True,
                enum_values=("both", "none", "request", "response"),
            ),
            BigipPropertySpec(
                name="internal-virtual",
                value_type="enum",
                in_sections=("service-endpoints",),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="order",
                value_type="integer",
                in_sections=("service-endpoints",),
            ),
            BigipPropertySpec(
                name="service-option",
                value_type="enum",
                in_sections=("service-endpoints",),
                enum_values=("mandatory", "optional"),
            ),
            BigipPropertySpec(
                name="steering-policy",
                value_type="reference",
                in_sections=("service-endpoints",),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="to-endpoint",
                value_type="reference",
                in_sections=("service-endpoints",),
            ),
        ),
    )
