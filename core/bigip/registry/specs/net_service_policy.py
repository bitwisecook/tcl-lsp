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
            "net_service_policy",
            module="net",
            object_types=("service-policy",),
        ),
        header_types=(("net", "service-policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="port-misuse-policy",
                value_type="reference",
                references=("security_firewall_port_misuse_policy",),
            ),
            BigipPropertySpec(
                name="timer-policy",
                value_type="reference",
                references=("net_timer_policy",),
            ),
        ),
    )
